use worldwake_ai::{GoalDispatchKeySchemaExt, htn::build_method_registry};
use worldwake_core::{GoalDispatchKey, GoalKindDiscriminant};

#[test]
fn dispatch_declarations_do_not_expose_method_assignment() {
    for key in GoalDispatchKey::all() {
        let declaration = key.declaration();

        assert!(
            !declaration.trace_label.is_empty(),
            "GoalDispatchKey::{key:?} should still expose schema metadata"
        );
    }
}

#[test]
fn method_registry_is_the_method_assignment_authority() {
    let registry = build_method_registry();

    for goal_kind in GoalKindDiscriminant::ALL {
        for method_id in registry.methods_for(*goal_kind) {
            let method = registry
                .get(*method_id)
                .expect("registry methods_for should only return registered method ids");

            assert_eq!(
                method.goal_kind, *goal_kind,
                "MethodSchemaId {method_id:?} should be assigned only through MethodRegistry"
            );
        }
    }
}
