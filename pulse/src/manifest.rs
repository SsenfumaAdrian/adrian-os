//! Service manifests and start-order resolution ("boot service graph"
//! and "dependency resolution" from Pulse's README).

use std::collections::{HashMap, HashSet};

/// A service's static definition: its name and what it depends on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceManifest {
    pub name: String,
    pub dependencies: Vec<String>,
}

impl ServiceManifest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dependencies: Vec::new(),
        }
    }

    /// Builder-style: `ServiceManifest::new("x").depends_on("a").depends_on("b")`.
    pub fn depends_on(mut self, dependency: impl Into<String>) -> Self {
        self.dependencies.push(dependency.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionError {
    /// A service depends on a name that isn't in the manifest set --
    /// reported before any ordering is attempted, since this is
    /// almost always a typo or a missing manifest, not a genuine
    /// cycle, and deserves a clearer error than treating it as one.
    UnknownDependency { service: String, missing: String },
    /// The dependency graph has a cycle: these services can never
    /// have a valid start order. Carries whichever service names were
    /// still unresolved when nothing further could proceed, sorted --
    /// enough to act on without claiming more precision (like the
    /// exact cycle path) than a straightforward check actually
    /// provides.
    Cycle(Vec<String>),
}

/// Compute a valid start order: every service appears after
/// everything it depends on. Kahn's algorithm -- repeatedly pull out
/// every service with no unresolved dependencies left, in
/// deterministic name-sorted order so the result doesn't depend on
/// hash-map iteration order, which Rust deliberately randomizes.
pub fn resolve_start_order(services: &[ServiceManifest]) -> Result<Vec<String>, ResolutionError> {
    let names: HashSet<&str> = services.iter().map(|s| s.name.as_str()).collect();

    for service in services {
        for dep in &service.dependencies {
            if !names.contains(dep.as_str()) {
                return Err(ResolutionError::UnknownDependency {
                    service: service.name.clone(),
                    missing: dep.clone(),
                });
            }
        }
    }

    let mut remaining: HashMap<&str, &ServiceManifest> =
        services.iter().map(|s| (s.name.as_str(), s)).collect();
    let mut resolved: HashSet<String> = HashSet::new();
    let mut order: Vec<String> = Vec::new();

    while !remaining.is_empty() {
        let mut ready: Vec<&str> = remaining
            .values()
            .filter(|s| s.dependencies.iter().all(|d| resolved.contains(d)))
            .map(|s| s.name.as_str())
            .collect();
        ready.sort();

        if ready.is_empty() {
            let mut stuck: Vec<String> = remaining.keys().map(|s| s.to_string()).collect();
            stuck.sort();
            return Err(ResolutionError::Cycle(stuck));
        }

        for name in ready {
            resolved.insert(name.to_string());
            order.push(name.to_string());
            remaining.remove(name);
        }
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(name: &str, deps: &[&str]) -> ServiceManifest {
        let mut manifest = ServiceManifest::new(name);
        for dep in deps {
            manifest = manifest.depends_on(*dep);
        }
        manifest
    }

    fn position(order: &[String], name: &str) -> usize {
        order.iter().position(|s| s.as_str() == name).unwrap()
    }

    #[test]
    fn empty_service_list_resolves_to_empty_order() {
        assert_eq!(resolve_start_order(&[]), Ok(vec![]));
    }

    #[test]
    fn independent_services_all_resolve_in_name_order() {
        let services = vec![service("c", &[]), service("a", &[]), service("b", &[])];
        assert_eq!(
            resolve_start_order(&services),
            Ok(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn linear_chain_resolves_in_dependency_order() {
        let services = vec![service("c", &["b"]), service("a", &[]), service("b", &["a"])];
        assert_eq!(
            resolve_start_order(&services),
            Ok(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn diamond_dependency_resolves_with_both_branches_before_the_join() {
        let services = vec![
            service("a", &[]),
            service("d", &["b", "c"]),
            service("b", &["a"]),
            service("c", &["a"]),
        ];
        let order = resolve_start_order(&services).unwrap();

        assert!(position(&order, "a") < position(&order, "b"));
        assert!(position(&order, "a") < position(&order, "c"));
        assert!(position(&order, "b") < position(&order, "d"));
        assert!(position(&order, "c") < position(&order, "d"));
    }

    #[test]
    fn direct_cycle_is_detected() {
        let services = vec![service("a", &["b"]), service("b", &["a"])];
        assert_eq!(
            resolve_start_order(&services),
            Err(ResolutionError::Cycle(vec!["a".to_string(), "b".to_string()]))
        );
    }

    #[test]
    fn self_dependency_is_detected_as_a_cycle() {
        let services = vec![service("a", &["a"])];
        assert_eq!(
            resolve_start_order(&services),
            Err(ResolutionError::Cycle(vec!["a".to_string()]))
        );
    }

    #[test]
    fn longer_cycle_is_detected() {
        // a -> b -> c -> a
        let services = vec![service("a", &["b"]), service("b", &["c"]), service("c", &["a"])];
        assert!(matches!(
            resolve_start_order(&services),
            Err(ResolutionError::Cycle(_))
        ));
    }

    #[test]
    fn dependency_on_an_unknown_service_is_reported_clearly() {
        let services = vec![service("a", &["ghost"])];
        assert_eq!(
            resolve_start_order(&services),
            Err(ResolutionError::UnknownDependency {
                service: "a".to_string(),
                missing: "ghost".to_string(),
            })
        );
    }

    #[test]
    fn unknown_dependency_is_reported_even_if_a_cycle_also_exists_elsewhere() {
        // Validation runs before cycle detection, so this reports the
        // missing dependency rather than attempting to resolve order
        // and reporting a cycle instead.
        let services = vec![service("a", &["b", "ghost"]), service("b", &["a"])];
        assert_eq!(
            resolve_start_order(&services),
            Err(ResolutionError::UnknownDependency {
                service: "a".to_string(),
                missing: "ghost".to_string(),
            })
        );
    }

    #[test]
    fn builder_accumulates_multiple_dependencies_in_order_added() {
        let manifest = ServiceManifest::new("x").depends_on("a").depends_on("b");
        assert_eq!(manifest.dependencies, vec!["a".to_string(), "b".to_string()]);
    }
}
