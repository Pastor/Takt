use grammar::parse;
use grammar::parser::ast::{InlineFormulaDefine, ModelElement, Statement};

#[test]
fn test_parse_ltl_formula() {
    let src = r#"
model M {
    always {
        :[LTL] X a;
        :[LTL] G (a & b) -> F c;
        :[LTL] a U b;
        :[LTL] a R b;
    }
}
"#;
    let (model, _) = parse(src, 0).unwrap();
    let elements = &model.elements;
    let model_element = elements
        .iter()
        .find(|e| matches!(e, ModelElement::Model(_)))
        .unwrap();
    if let ModelElement::Model(m) = model_element {
        let always = m
            .elements
            .iter()
            .find(|e| matches!(e, ModelElement::NamedBlockCode(_)))
            .unwrap();
        if let ModelElement::NamedBlockCode(nb) = always {
            if let Statement::Block { statements, .. } = &nb.statement {
                assert_eq!(statements.len(), 4);
                for stmt in statements {
                    assert!(matches!(stmt, Statement::InlineFormula(_)));
                    if let Statement::InlineFormula(f) = stmt {
                        assert!(matches!(**f, InlineFormulaDefine::Ltl { .. }));
                    }
                }
            }
        }
    }
}

#[test]
fn test_parse_guard_formula() {
    let src = r#"
model M {
    always {
        : a > 0;
        :[Guard] b < 10;
    }
}
"#;
    let (model, _) = parse(src, 0).unwrap();
    let elements = &model.elements;
    let model_element = elements
        .iter()
        .find(|e| matches!(e, ModelElement::Model(_)))
        .unwrap();
    if let ModelElement::Model(m) = model_element {
        let always = m
            .elements
            .iter()
            .find(|e| matches!(e, ModelElement::NamedBlockCode(_)))
            .unwrap();
        if let ModelElement::NamedBlockCode(nb) = always {
            if let Statement::Block { statements, .. } = &nb.statement {
                assert_eq!(statements.len(), 2);
                for stmt in statements {
                    assert!(matches!(stmt, Statement::InlineFormula(_)));
                    if let Statement::InlineFormula(f) = stmt {
                        assert!(matches!(**f, InlineFormulaDefine::Guard { .. }));
                    }
                }
            }
        }
    }
}

#[test]
fn test_parse_mixed_formulas() {
    let src = r#"
model M {
    always {
        :[LTL] X End;
        : i > 0;
    }
}
"#;
    let (model, _) = parse(src, 0).unwrap();
    let elements = &model.elements;
    let model_element = elements
        .iter()
        .find(|e| matches!(e, ModelElement::Model(_)))
        .unwrap();
    if let ModelElement::Model(m) = model_element {
        let always = m
            .elements
            .iter()
            .find(|e| matches!(e, ModelElement::NamedBlockCode(_)))
            .unwrap();
        if let ModelElement::NamedBlockCode(nb) = always {
            if let Statement::Block { statements, .. } = &nb.statement {
                assert_eq!(statements.len(), 2);

                if let Statement::InlineFormula(f) = &statements[0] {
                    assert!(matches!(**f, InlineFormulaDefine::Ltl { .. }));
                } else {
                    panic!("Expected LTL formula");
                }

                if let Statement::InlineFormula(f) = &statements[1] {
                    assert!(matches!(**f, InlineFormulaDefine::Guard { .. }));
                } else {
                    panic!("Expected Guard formula");
                }
            }
        }
    }
}
