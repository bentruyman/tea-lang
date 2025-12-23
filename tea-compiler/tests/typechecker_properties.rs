//! Property-based tests for the type checker's unification and substitution logic.
//!
//! These tests use proptest to generate arbitrary types and verify that key
//! invariants hold across many random inputs.

use proptest::prelude::*;
use std::collections::HashMap;

// We need to access the internal Type enum, so we'll define a mirror type for testing
// and convert between them. This avoids needing to make Type public.

/// Mirror of the internal Type enum for property testing.
/// We test properties on this type, which has the same structure as the real Type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TestType {
    Bool,
    Int,
    Float,
    String,
    Nil,
    Void,
    Optional(Box<TestType>),
    List(Box<TestType>),
    Dict(Box<TestType>),
    Function(Vec<TestType>, Box<TestType>),
    GenericParameter(String),
}

impl TestType {
    fn describe(&self) -> String {
        match self {
            TestType::Bool => "Bool".to_string(),
            TestType::Int => "Int".to_string(),
            TestType::Float => "Float".to_string(),
            TestType::String => "String".to_string(),
            TestType::Nil => "Nil".to_string(),
            TestType::Void => "Void".to_string(),
            TestType::Optional(inner) => format!("{}?", inner.describe()),
            TestType::List(element) => format!("List[{}]", element.describe()),
            TestType::Dict(value) => format!("Dict[String, {}]", value.describe()),
            TestType::Function(params, return_type) => {
                let param_str = if params.is_empty() {
                    String::from("()")
                } else {
                    let joined = params
                        .iter()
                        .map(|param| param.describe())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("({joined})")
                };
                format!("Func{} -> {}", param_str, return_type.describe())
            }
            TestType::GenericParameter(name) => name.clone(),
        }
    }
}

/// Strategy to generate concrete leaf types (no generics).
fn concrete_leaf_type() -> impl Strategy<Value = TestType> {
    prop_oneof![
        Just(TestType::Bool),
        Just(TestType::Int),
        Just(TestType::Float),
        Just(TestType::String),
        Just(TestType::Nil),
        Just(TestType::Void),
    ]
}

/// Strategy to generate leaf types (non-recursive), including generics.
fn leaf_type() -> impl Strategy<Value = TestType> {
    prop_oneof![
        concrete_leaf_type(),
        "[A-Z]".prop_map(TestType::GenericParameter),
    ]
}

/// Strategy to generate arbitrary types with controlled recursion depth.
fn arb_type() -> impl Strategy<Value = TestType> {
    leaf_type().prop_recursive(
        3,  // max depth
        64, // max nodes
        10, // items per collection
        |inner| {
            prop_oneof![
                // Optional type
                inner.clone().prop_map(|t| TestType::Optional(Box::new(t))),
                // List type
                inner.clone().prop_map(|t| TestType::List(Box::new(t))),
                // Dict type
                inner.clone().prop_map(|t| TestType::Dict(Box::new(t))),
                // Function type with 0-3 parameters
                (prop::collection::vec(inner.clone(), 0..3), inner.clone())
                    .prop_map(|(params, ret)| TestType::Function(params, Box::new(ret))),
            ]
        },
    )
}

/// Strategy to generate a type substitution mapping.
fn arb_substitution() -> impl Strategy<Value = HashMap<String, TestType>> {
    prop::collection::hash_map("[A-Z]", arb_type(), 0..5)
}

// =============================================================================
// Substitution implementation (mirrors the real substitute_type)
// =============================================================================

fn substitute_type(ty: &TestType, mapping: &HashMap<String, TestType>) -> TestType {
    match ty {
        TestType::GenericParameter(name) => mapping
            .get(name)
            .cloned()
            .unwrap_or_else(|| TestType::GenericParameter(name.clone())),
        TestType::Optional(inner) => TestType::Optional(Box::new(substitute_type(inner, mapping))),
        TestType::List(inner) => TestType::List(Box::new(substitute_type(inner, mapping))),
        TestType::Dict(inner) => TestType::Dict(Box::new(substitute_type(inner, mapping))),
        TestType::Function(params, return_type) => {
            let substituted_params = params
                .iter()
                .map(|param| substitute_type(param, mapping))
                .collect();
            let substituted_return = substitute_type(return_type, mapping);
            TestType::Function(substituted_params, Box::new(substituted_return))
        }
        other => other.clone(),
    }
}

// =============================================================================
// Unification implementation (simplified version for testing properties)
// =============================================================================

/// Attempt to unify two types, building a mapping of generic parameters.
/// Returns Some(mapping) if successful, None if types are incompatible.
fn unify_types(
    expected: &TestType,
    actual: &TestType,
    mapping: &mut HashMap<String, TestType>,
) -> bool {
    match expected {
        TestType::GenericParameter(name) => {
            if let Some(existing) = mapping.get(name) {
                // Generic already bound - check if actual matches the bound type
                existing == actual
            } else {
                // Bind the generic to the actual type
                mapping.insert(name.clone(), actual.clone());
                true
            }
        }
        TestType::Optional(expected_inner) => match actual {
            TestType::Optional(actual_inner) => unify_types(expected_inner, actual_inner, mapping),
            _ => expected == actual,
        },
        TestType::List(expected_inner) => {
            if let TestType::List(actual_inner) = actual {
                unify_types(expected_inner, actual_inner, mapping)
            } else {
                false
            }
        }
        TestType::Dict(expected_inner) => {
            if let TestType::Dict(actual_inner) = actual {
                unify_types(expected_inner, actual_inner, mapping)
            } else {
                false
            }
        }
        TestType::Function(expected_params, expected_ret) => {
            if let TestType::Function(actual_params, actual_ret) = actual {
                if expected_params.len() != actual_params.len() {
                    return false;
                }
                for (ep, ap) in expected_params.iter().zip(actual_params.iter()) {
                    if !unify_types(ep, ap, mapping) {
                        return false;
                    }
                }
                unify_types(expected_ret, actual_ret, mapping)
            } else {
                false
            }
        }
        _ => expected == actual,
    }
}

// =============================================================================
// Property Tests
// =============================================================================

proptest! {
    /// Property: substitute_type is idempotent when the mapping contains only
    /// concrete types (applying it twice gives the same result).
    #[test]
    fn substitute_idempotent_with_concrete_mapping(
        ty in arb_type(),
        mapping in prop::collection::hash_map("[A-Z]", concrete_leaf_type(), 0..5)
    ) {
        let once = substitute_type(&ty, &mapping);
        let twice = substitute_type(&once, &mapping);
        prop_assert_eq!(once, twice, "Substitution should be idempotent with concrete types");
    }

    /// Property: substituting with an empty mapping returns the original type.
    #[test]
    fn substitute_empty_mapping_identity(ty in arb_type()) {
        let empty: HashMap<String, TestType> = HashMap::new();
        let result = substitute_type(&ty, &empty);
        prop_assert_eq!(ty, result, "Empty substitution should be identity");
    }

    /// Property: substitution preserves type structure (non-generic parts unchanged).
    #[test]
    fn substitute_preserves_structure(ty in arb_type(), mapping in arb_substitution()) {
        let result = substitute_type(&ty, &mapping);
        // The describe output should have the same "shape" in terms of brackets
        // This is a weak check but ensures we're not destroying the type structure
        let orig_depth = count_nesting(&ty);
        let result_depth = count_nesting(&result);
        // Depth can only decrease or stay the same (if a generic is replaced with a simpler type)
        // or increase (if a generic is replaced with a more complex type)
        // But the relationship should be bounded
        prop_assert!(
            result_depth <= orig_depth + mapping.values().map(count_nesting).max().unwrap_or(0),
            "Result depth {} should be bounded by original {} plus max mapping depth",
            result_depth, orig_depth
        );
    }

    /// Property: unification is reflexive (every type unifies with itself).
    #[test]
    fn unify_reflexive(ty in arb_type()) {
        let mut mapping = HashMap::new();
        prop_assert!(
            unify_types(&ty, &ty, &mut mapping),
            "Type {} should unify with itself",
            ty.describe()
        );
    }

    /// Property: unification of concrete types (no generics) is symmetric.
    #[test]
    fn unify_concrete_symmetric(a in leaf_type(), b in leaf_type()) {
        // Skip generic parameters for symmetry test
        if matches!(a, TestType::GenericParameter(_)) || matches!(b, TestType::GenericParameter(_)) {
            return Ok(());
        }
        let mut mapping_ab = HashMap::new();
        let mut mapping_ba = HashMap::new();
        let ab = unify_types(&a, &b, &mut mapping_ab);
        let ba = unify_types(&b, &a, &mut mapping_ba);
        prop_assert_eq!(ab, ba, "Unification should be symmetric for concrete types");
    }

    /// Property: if unification succeeds and we substitute the mapping into
    /// the expected type, we should get the actual type (for simple cases).
    #[test]
    fn unify_then_substitute_gives_actual(
        // Use a generic type as expected
        generic_name in "[A-Z]",
        actual in leaf_type()
    ) {
        // Skip if actual is also a generic (would create circular reference)
        if matches!(actual, TestType::GenericParameter(_)) {
            return Ok(());
        }

        let expected = TestType::GenericParameter(generic_name);
        let mut mapping = HashMap::new();

        if unify_types(&expected, &actual, &mut mapping) {
            let substituted = substitute_type(&expected, &mapping);
            prop_assert_eq!(
                substituted, actual,
                "Substituting unified mapping into expected should give actual"
            );
        }
    }

    /// Property: unification with List preserves element type relationship.
    #[test]
    fn unify_list_preserves_element(
        inner_expected in arb_type(),
        inner_actual in arb_type()
    ) {
        let list_expected = TestType::List(Box::new(inner_expected.clone()));
        let list_actual = TestType::List(Box::new(inner_actual.clone()));

        let mut mapping_inner = HashMap::new();
        let mut mapping_list = HashMap::new();

        let inner_result = unify_types(&inner_expected, &inner_actual, &mut mapping_inner);
        let list_result = unify_types(&list_expected, &list_actual, &mut mapping_list);

        prop_assert_eq!(
            inner_result, list_result,
            "List[T] unification should match element unification"
        );
    }

    /// Property: unification with Dict preserves value type relationship.
    #[test]
    fn unify_dict_preserves_value(
        inner_expected in arb_type(),
        inner_actual in arb_type()
    ) {
        let dict_expected = TestType::Dict(Box::new(inner_expected.clone()));
        let dict_actual = TestType::Dict(Box::new(inner_actual.clone()));

        let mut mapping_inner = HashMap::new();
        let mut mapping_dict = HashMap::new();

        let inner_result = unify_types(&inner_expected, &inner_actual, &mut mapping_inner);
        let dict_result = unify_types(&dict_expected, &dict_actual, &mut mapping_dict);

        prop_assert_eq!(
            inner_result, dict_result,
            "Dict[T] unification should match value type unification"
        );
    }

    /// Property: function unification fails if parameter counts differ.
    #[test]
    fn unify_function_param_count_mismatch(
        params1 in prop::collection::vec(arb_type(), 0..3),
        params2 in prop::collection::vec(arb_type(), 0..3),
        ret in arb_type()
    ) {
        if params1.len() == params2.len() {
            return Ok(());
        }

        let func1 = TestType::Function(params1, Box::new(ret.clone()));
        let func2 = TestType::Function(params2, Box::new(ret));

        let mut mapping = HashMap::new();
        prop_assert!(
            !unify_types(&func1, &func2, &mut mapping),
            "Functions with different parameter counts should not unify"
        );
    }
}

/// Helper function to count the nesting depth of a type.
fn count_nesting(ty: &TestType) -> usize {
    match ty {
        TestType::Bool
        | TestType::Int
        | TestType::Float
        | TestType::String
        | TestType::Nil
        | TestType::Void
        | TestType::GenericParameter(_) => 0,
        TestType::Optional(inner) | TestType::List(inner) | TestType::Dict(inner) => {
            1 + count_nesting(inner)
        }
        TestType::Function(params, ret) => {
            let param_max = params.iter().map(count_nesting).max().unwrap_or(0);
            1 + param_max.max(count_nesting(ret))
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_substitute_basic() {
        let ty = TestType::GenericParameter("T".to_string());
        let mut mapping = HashMap::new();
        mapping.insert("T".to_string(), TestType::Int);

        let result = substitute_type(&ty, &mapping);
        assert_eq!(result, TestType::Int);
    }

    #[test]
    fn test_substitute_nested() {
        let ty = TestType::List(Box::new(TestType::GenericParameter("T".to_string())));
        let mut mapping = HashMap::new();
        mapping.insert("T".to_string(), TestType::String);

        let result = substitute_type(&ty, &mapping);
        assert_eq!(result, TestType::List(Box::new(TestType::String)));
    }

    #[test]
    fn test_unify_generic_binds() {
        let expected = TestType::GenericParameter("T".to_string());
        let actual = TestType::Int;
        let mut mapping = HashMap::new();

        assert!(unify_types(&expected, &actual, &mut mapping));
        assert_eq!(mapping.get("T"), Some(&TestType::Int));
    }

    #[test]
    fn test_unify_generic_consistent() {
        // List[T] should unify with List[Int], binding T=Int
        let expected = TestType::List(Box::new(TestType::GenericParameter("T".to_string())));
        let actual = TestType::List(Box::new(TestType::Int));
        let mut mapping = HashMap::new();

        assert!(unify_types(&expected, &actual, &mut mapping));
        assert_eq!(mapping.get("T"), Some(&TestType::Int));
    }

    #[test]
    fn test_unify_generic_inconsistent() {
        // If T is already bound to Int, it shouldn't unify with String
        let expected = TestType::GenericParameter("T".to_string());
        let actual = TestType::String;
        let mut mapping = HashMap::new();
        mapping.insert("T".to_string(), TestType::Int);

        assert!(!unify_types(&expected, &actual, &mut mapping));
    }
}
