use crate::ast::*;
use crate::atom::{HttpStatus, HttpStatusRange, Method, Primitive};
use crate::parse;
use enum_map::enum_map;

#[derive(Clone, Debug, PartialEq)]
struct TestExpr(NodeExpr<TestExpr>);

impl From<NodeExpr<TestExpr>> for TestExpr {
    fn from(e: NodeExpr<TestExpr>) -> Self {
        TestExpr(e)
    }
}

impl AsRefNode for TestExpr {
    fn as_node(&self) -> &NodeExpr<TestExpr> {
        &self.0
    }
}

impl AsMutNode for TestExpr {
    fn as_node_mut(&mut self) -> &mut NodeExpr<TestExpr> {
        &mut self.0
    }
}

type Program = crate::ast::Program<TestExpr>;

#[test]
fn parse_variable_decl() {
    let d: Program = parse("let a = num;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        assert_eq!(decl.name.as_ref(), "a");
        assert_eq!(
            *decl.expr.as_node().as_expr(),
            Expr::Prim(Primitive::Number)
        );
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_assignment() {
    let d: Program = parse("let a = b;").expect("parsing failed");
    assert_eq!(d.stmts.len(), 1);
}

#[test]
fn parse_array() {
    let d: Program = parse("let a = [str];").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        if let Expr::Array(array) = decl.expr.as_node().as_expr() {
            assert_eq!(
                *array.item.as_node().as_expr(),
                Expr::Prim(Primitive::String)
            );
        } else {
            panic!("expected array expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_root_uri() {
    let d: Program = parse("let a = /;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        if let Expr::Uri(uri) = decl.expr.as_node().as_expr() {
            assert_eq!(uri.path.len(), 1);
            assert_eq!(*uri.path.first().unwrap(), UriSegment::root());
        } else {
            panic!("expected uri expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_template_uri() {
    let d: Program = parse("let a = /x/{ 'y str }/z/;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        if let Expr::Uri(uri) = decl.expr.as_node().as_expr() {
            assert_eq!(uri.path.len(), 3);
            assert_eq!(*uri.path.get(0).unwrap(), UriSegment::Literal("x".into()));
            assert_eq!(*uri.path.get(2).unwrap(), UriSegment::Literal("z".into()));
            if let UriSegment::Variable(var) = uri.path.get(1).unwrap() {
                if let Expr::Property(prop) = var.as_node().as_expr() {
                    assert_eq!(prop.name.as_ref(), "y");
                    assert_eq!(*prop.val.as_node().as_expr(), Expr::Prim(Primitive::String));
                } else {
                    panic!("expected property expression");
                }
            } else {
                panic!("expected uri segment variable");
            }
        } else {
            panic!("expected uri expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_composite_relation() {
    let code = r#"
        let a = / (
            patch, put : <{}> -> <{}>,
            get               -> <{}>
        );
    "#;
    let d: Program = parse(code).expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        if let Expr::Rel(rel) = decl.expr.as_node().as_expr() {
            assert_eq!(rel.xfers.len(), 2);

            if let (Expr::Xfer(x0), Expr::Xfer(x1)) = (
                rel.xfers[0].as_node().as_expr(),
                rel.xfers[1].as_node().as_expr(),
            ) {
                assert_eq!(
                    x0.methods,
                    enum_map! {
                        Method::Put => true,
                        Method::Patch => true,
                        _ => false,
                    }
                );
                assert_eq!(
                    x1.methods,
                    enum_map! {
                        Method::Get => true,
                        _ => false,
                    }
                );
            }
        } else {
            panic!("expected relation expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_simple_relation() {
    let d: Program = parse("let a = / ( put : <{}> -> <{}> );").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        if let Expr::Rel(rel) = decl.expr.as_node().as_expr() {
            assert_eq!(
                *rel.uri.as_node().as_expr(),
                Expr::Uri(Uri {
                    path: vec![UriSegment::root()],
                    params: None,
                })
            );

            assert_eq!(rel.xfers.len(), 1);

            if let Expr::Xfer(xfer) = rel.xfers[0].as_node().as_expr() {
                if let Some(domain) = &xfer.domain {
                    if let Expr::Content(cnt) = domain.as_node().as_expr() {
                        assert_eq!(
                            *cnt.schema.as_ref().unwrap().as_node().as_expr(),
                            Expr::Object(Default::default())
                        );
                    } else {
                        panic!("expected content expression");
                    }
                } else {
                    panic!("expected domain expression");
                }

                if let Expr::Content(cnt) = xfer.ranges.as_node().as_expr() {
                    assert_eq!(
                        *cnt.schema.as_ref().unwrap().as_node().as_expr(),
                        Expr::Object(Default::default())
                    );
                } else {
                    panic!("expected content expression");
                }
            } else {
                panic!("expected transfer expression");
            }
        } else {
            panic!("expected relation expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_any_type() {
    let d: Program = parse("let a = {} ~ uri ~ bool;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        if let Expr::Op(VariadicOp {
            op: Operator::Any,
            exprs,
        }) = decl.expr.as_node().as_expr()
        {
            assert_eq!(exprs.len(), 3);
        } else {
            panic!("expected untyped alternative operation");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_application() {
    let d: Program = parse("let a = f num {} uri;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        if let Expr::App(Application { name, args }) = decl.expr.as_node().as_expr() {
            assert_eq!(name.as_ref(), "f");
            assert_eq!(args.len(), 3);
        } else {
            panic!("expected function application");
        }
    }
}

#[test]
fn parse_lambda_decl() {
    let d: Program = parse("let f x y z = num;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        assert_eq!(decl.name.as_ref(), "f");
        if let Expr::Lambda(Lambda {
            body,
            bindings: args,
        }) = decl.expr.as_node().as_expr()
        {
            let bindings: Vec<_> = args
                .iter()
                .filter_map(|a| match a.as_node().as_expr() {
                    Expr::Binding(b) => Some(b.as_ref()),
                    _ => None,
                })
                .collect();
            assert_eq!(bindings, vec!["x", "y", "z"]);
            assert_eq!(*body.as_node().as_expr(), Expr::Prim(Primitive::Number));
        } else {
            panic!("expected lambda expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_annotation() {
    let code = r#"
        # description: "some identifer"
        let id = num;
        # description: "some record"
        let r = {};
        let a = /{ 'n id } ( put : <r> -> <r> );
    "#;
    let d: Program = parse(code).expect("parsing failed");

    assert_eq!(d.stmts.len(), 5);

    if let Statement::Ann(ann) = d.stmts.get(0).unwrap() {
        assert_eq!(ann.text, r#" description: "some identifer""#);
    } else {
        panic!("expected annotation");
    }
    if let Statement::Ann(ann) = d.stmts.get(2).unwrap() {
        assert_eq!(ann.text, r#" description: "some record""#);
    } else {
        panic!("expected annotation");
    }
}

#[test]
fn parse_empty_content() {
    let d: Program = parse("let c = <>;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        assert_eq!(decl.name.as_ref(), "c");
        if let Expr::Content(cnt) = decl.expr.as_node().as_expr() {
            assert!(cnt.schema.is_none());
        } else {
            panic!("expected content expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_complete_content() {
    let code = r#"
        let c = <
            media = "application/json",
            status = 200,
            headers = {},
            {}
        >;
    "#;
    let d: Program = parse(code).expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        assert_eq!(decl.name.as_ref(), "c");
        if let Expr::Content(cnt) = decl.expr.as_node().as_expr() {
            assert!(cnt.schema.is_some());
            assert_eq!(
                *cnt.status
                    .as_ref()
                    .expect("expected status")
                    .as_node()
                    .as_expr(),
                Expr::Lit(Literal::Number(200))
            );
            assert_eq!(
                *cnt.media
                    .as_ref()
                    .expect("expected media")
                    .as_node()
                    .as_expr(),
                Expr::Lit(Literal::Text("application/json".into()))
            );
            assert_eq!(
                *cnt.headers
                    .as_ref()
                    .expect("expected headers")
                    .as_node()
                    .as_expr(),
                Expr::Object(Object::default())
            );
        } else {
            panic!("expected content expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_status_range() {
    let d: Program = parse("let c = <status=4XX,{}>;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        assert_eq!(decl.name.as_ref(), "c");
        if let Expr::Content(cnt) = decl.expr.as_node().as_expr() {
            assert_eq!(
                *cnt.status
                    .as_ref()
                    .expect("expected status")
                    .as_node()
                    .as_expr(),
                Expr::Lit(Literal::Status(HttpStatus::Range(
                    HttpStatusRange::ClientError
                )))
            );
        } else {
            panic!("expected content expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_inline_annotation() {
    let d: Program = parse(r#"let a = num `title: "number"`;"#).expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        let ann = decl
            .expr
            .as_node()
            .ann
            .as_ref()
            .expect("expected annotation");
        assert_eq!(ann.text, r#"title: "number""#);
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_import() {
    let d: Program = parse(r#"use "module";"#).expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Use(imp) = d.stmts.first().unwrap() {
        assert_eq!(imp.module, "module");
    } else {
        panic!("expected import");
    }
}

#[test]
fn parse_uri_params() {
    let d: Program = parse("let a = /x/{ 'y str }/z?{ 'q str, 'n num };").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    let s = d.stmts.first().unwrap();

    if let Statement::Decl(decl) = s {
        if let Expr::Uri(uri) = decl.expr.as_node().as_expr() {
            let params = uri.params.as_ref().expect("expected params");
            if let Expr::Object(obj) = params.as_node().as_expr() {
                assert_eq!(obj.props.len(), 2)
            } else {
                panic!("expected object expression");
            }
        } else {
            panic!("expected uri expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_transfer_params() {
    let d: Program = parse("let a = get { 'q str, 'n num } -> {};").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        if let Expr::Xfer(xfer) = decl.expr.as_node().as_expr() {
            let params = xfer.params.as_ref().expect("expected params");
            if let Expr::Object(obj) = params.as_node().as_expr() {
                assert_eq!(obj.props.len(), 2)
            } else {
                panic!("expected object expression");
            }
        } else {
            panic!("expected xfer expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_nondeterministic_transfer() {
    let d: Program = parse("let a = get -> <{}> :: <{}>;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        if let Expr::Xfer(xfer) = decl.expr.as_node().as_expr() {
            if let Expr::Op(op) = xfer.ranges.as_node().as_expr() {
                assert_eq!(op.op, Operator::Range);
                assert_eq!(op.exprs.len(), 2);
            } else {
                panic!("expected ranges");
            }
        } else {
            panic!("expected xfer expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_property_decl() {
    let d: Program = parse("let a = 'q str;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        if let Expr::Property(prop) = decl.expr.as_node().as_expr() {
            assert_eq!(prop.name.as_ref(), "q");
            assert_eq!(*prop.val.as_node().as_expr(), Expr::Prim(Primitive::String));
        } else {
            panic!("expected property expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_literal_number_decl() {
    let d: Program = parse("let a = 404;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        if let Expr::Lit(Literal::Number(n)) = decl.expr.as_node().as_expr() {
            assert_eq!(*n, 404);
        } else {
            panic!("expected literal number expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_literal_text_decl() {
    let d: Program = parse(r#"let a = "application/json";"#).expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        if let Expr::Lit(Literal::Text(t)) = decl.expr.as_node().as_expr() {
            assert_eq!(t.as_ref(), "application/json");
        } else {
            panic!("expected literal number expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_literal_status_decl() {
    let d: Program = parse("let a = 4XX;").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        if let Expr::Lit(Literal::Status(s)) = decl.expr.as_node().as_expr() {
            assert_eq!(*s, HttpStatus::Range(HttpStatusRange::ClientError));
        } else {
            panic!("expected literal status expression");
        }
    } else {
        panic!("expected declaration");
    }
}

#[test]
fn parse_reference_decl() {
    let d: Program = parse("let @a = {};").expect("parsing failed");

    assert_eq!(d.stmts.len(), 1);

    if let Statement::Decl(decl) = d.stmts.first().unwrap() {
        assert!(decl.name.is_reference());
    } else {
        panic!("expected declaration");
    }
}
