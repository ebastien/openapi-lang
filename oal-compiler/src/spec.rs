use crate::annotation::Annotated;
use crate::errors::{Error, Kind, Result};
use crate::module::ModuleSet;
use crate::node::NodeRef;
use crate::scan::Scan;
use crate::scope::Env;
use enum_map::EnumMap;
use indexmap::IndexMap;
use oal_syntax::ast::AsExpr;
use oal_syntax::atom::{HttpStatus, Ident, Text};
use oal_syntax::{ast, atom};
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq)]
pub enum UriSegment {
    Literal(Text),
    Variable(Box<Property>),
}

impl<T> TryFrom<&ast::UriSegment<T>> for UriSegment
where
    T: AsExpr + Annotated,
{
    type Error = Error;

    fn try_from(s: &ast::UriSegment<T>) -> Result<Self> {
        match s {
            ast::UriSegment::Literal(l) => Ok(UriSegment::Literal(l.clone())),
            ast::UriSegment::Variable(p) => {
                Property::try_from(p).map(|p| UriSegment::Variable(Box::new(p)))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Uri {
    pub path: Vec<UriSegment>,
    pub params: Option<Object>,
    pub example: Option<String>,
}

impl Uri {
    pub fn pattern(&self) -> String {
        self.path
            .iter()
            .map(|s| match s {
                UriSegment::Literal(l) => format!("/{}", l),
                UriSegment::Variable(t) => format!("/{{{}}}", t.name),
            })
            .collect()
    }

    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        if let ast::Expr::Uri(uri) = e.as_node().as_expr() {
            let path = uri
                .path
                .iter()
                .map(UriSegment::try_from)
                .collect::<Result<Vec<UriSegment>>>()?;
            let params = if let Some(p) = &uri.params {
                let obj = Object::try_from(p.as_ref())?;
                Some(obj)
            } else {
                None
            };
            let ann = e.annotation();
            let example = ann.and_then(|a| a.get_string("example"));
            Ok(Uri {
                path,
                params,
                example,
            })
        } else {
            Err(Error::new(Kind::UnexpectedExpression, "not a URI").with(e))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Array {
    pub item: Schema,
}

impl Array {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        if let ast::Expr::Array(a) = e.as_node().as_expr() {
            Schema::try_from(a.item.as_ref()).map(|item| Array { item })
        } else {
            Err(Error::new(Kind::UnexpectedExpression, "not an array").with(e))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariadicOp {
    pub op: ast::Operator,
    pub schemas: Vec<Schema>,
}

impl VariadicOp {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        if let ast::Expr::Op(op) = e.as_node().as_expr() {
            let schemas: Result<Vec<_>> = op.exprs.iter().map(Schema::try_from).collect();
            schemas.map(|schemas| VariadicOp { op: op.op, schemas })
        } else {
            Err(Error::new(Kind::UnexpectedExpression, "not an operation").with(e))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    pub expr: SchemaExpr,
    pub desc: Option<String>,
    pub title: Option<String>,
    pub required: Option<bool>,
}

impl Schema {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        let expr = SchemaExpr::try_from(e)?;
        let ann = e.annotation();
        let desc = ann.and_then(|a| a.get_string("description"));
        let title = ann.and_then(|a| a.get_string("title"));
        let required = ann.and_then(|a| a.get_bool("required"));
        Ok(Schema {
            expr,
            desc,
            title,
            required,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimNumber {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub multiple_of: Option<f64>,
    pub example: Option<f64>,
}

impl PrimNumber {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        let ann = e.annotation();
        let minimum = ann.and_then(|a| a.get_num("minimum"));
        let maximum = ann.and_then(|a| a.get_num("maximum"));
        let multiple_of = ann.and_then(|a| a.get_num("multipleOf"));
        let example = ann.and_then(|a| a.get_num("example"));
        Ok(PrimNumber {
            minimum,
            maximum,
            multiple_of,
            example,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimString {
    pub pattern: Option<String>,
    pub enumeration: Vec<String>,
    pub example: Option<String>,
}

impl PrimString {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        let ann = e.annotation();
        let pattern = ann.and_then(|a| a.get_string("pattern"));
        let enumeration = ann.and_then(|a| a.get_enum("enum")).unwrap_or_default();
        let example = ann.and_then(|a| a.get_string("example"));
        Ok(PrimString {
            pattern,
            enumeration,
            example,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimBoolean {}

impl PrimBoolean {
    fn try_from<T: AsExpr + Annotated>(_: &T) -> Result<Self> {
        Ok(PrimBoolean {})
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimInteger {
    pub minimum: Option<i64>,
    pub maximum: Option<i64>,
    pub multiple_of: Option<i64>,
    pub example: Option<i64>,
}

impl PrimInteger {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        let ann = e.annotation();
        let minimum = ann.and_then(|a| a.get_int("minimum"));
        let maximum = ann.and_then(|a| a.get_int("maximum"));
        let multiple_of = ann.and_then(|a| a.get_int("multipleOf"));
        let example = ann.and_then(|a| a.get_int("example"));
        Ok(PrimInteger {
            minimum,
            maximum,
            multiple_of,
            example,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SchemaExpr {
    Num(PrimNumber),
    Str(PrimString),
    Bool(PrimBoolean),
    Int(PrimInteger),
    Rel(Box<Relation>),
    Uri(Uri),
    Array(Box<Array>),
    Object(Object),
    Op(VariadicOp),
    Ref(Ident),
}

impl SchemaExpr {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        let node = e.as_node();
        let span = node.span;
        match node.as_expr() {
            ast::Expr::Prim(atom::Primitive::Number) => {
                PrimNumber::try_from(e).map(SchemaExpr::Num)
            }
            ast::Expr::Prim(atom::Primitive::String) => {
                PrimString::try_from(e).map(SchemaExpr::Str)
            }
            ast::Expr::Prim(atom::Primitive::Boolean) => {
                PrimBoolean::try_from(e).map(SchemaExpr::Bool)
            }
            ast::Expr::Prim(atom::Primitive::Integer) => {
                PrimInteger::try_from(e).map(SchemaExpr::Int)
            }
            ast::Expr::Rel(_) => Relation::try_from(e).map(|r| SchemaExpr::Rel(Box::new(r))),
            ast::Expr::Uri(_) => Uri::try_from(e).map(SchemaExpr::Uri),
            ast::Expr::Array(_) => Array::try_from(e).map(|a| SchemaExpr::Array(Box::new(a))),
            ast::Expr::Object(_) => Object::try_from(e).map(SchemaExpr::Object),
            ast::Expr::Op(_) => VariadicOp::try_from(e).map(SchemaExpr::Op),
            ast::Expr::Var(v) if v.is_reference() => Ok(SchemaExpr::Ref(v.clone())),
            _ => Err(Error::new(Kind::UnexpectedExpression, "expected schema-like").with(e)),
        }
        .map_err(|err| err.at(span))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub name: Ident,
    pub schema: Schema,
    /// The property description when used as a parameter
    pub desc: Option<String>,
    /// Whether the property is required when used as a parameter
    pub required: Option<bool>,
}

impl Property {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        if let ast::Expr::Property(prop) = e.as_node().as_expr() {
            let name = prop.name.clone();
            let schema = Schema::try_from(prop.val.as_ref())?;
            let ann = e.annotation();
            let desc = ann.and_then(|a| a.get_string("description"));
            let required = ann.and_then(|a| a.get_bool("required"));
            Ok(Property {
                name,
                schema,
                desc,
                required,
            })
        } else {
            Err(Error::new(Kind::UnexpectedExpression, "not a property").with(e))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Object {
    pub props: Vec<Property>,
}

impl Object {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        if let ast::Expr::Object(o) = e.as_node().as_expr() {
            let props: Result<Vec<_>> = o.props.iter().map(Property::try_from).collect();
            props.map(|props| Object { props })
        } else {
            Err(Error::new(Kind::UnexpectedExpression, "not an object").with(e))
        }
    }
}

fn try_into_status<T: AsExpr + Annotated>(e: &T) -> Result<HttpStatus> {
    match e.as_node().as_expr() {
        ast::Expr::Lit(ast::Literal::Status(s)) => Ok(*s),
        ast::Expr::Lit(ast::Literal::Number(n)) => {
            let s = HttpStatus::try_from(*n)?;
            Ok(s)
        }
        _ => Err(Error::new(Kind::UnexpectedExpression, "not a status expression").with(e)),
    }
}

fn try_into_media<T: AsExpr + Annotated>(e: &T) -> Result<MediaType> {
    match e.as_node().as_expr() {
        ast::Expr::Lit(ast::Literal::Text(t)) => Ok(t.as_ref().to_owned()),
        _ => Err(Error::new(Kind::UnexpectedExpression, "not a media expression").with(e)),
    }
}

pub type MediaType = String;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Content {
    pub schema: Option<Box<Schema>>,
    pub status: Option<HttpStatus>,
    pub media: Option<MediaType>,
    pub headers: Option<Object>,
    pub desc: Option<String>,
}

impl From<Schema> for Content {
    fn from(s: Schema) -> Self {
        let desc = s.desc.clone();
        let schema = Some(s.into());
        let status = None;
        let media = None;
        let headers = None;
        Content {
            schema,
            status,
            media,
            headers,
            desc,
        }
    }
}

impl Content {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        if let ast::Expr::Content(content) = e.as_node().as_expr() {
            let schema = match &content.schema {
                Some(s) => Schema::try_from(s.as_ref()).map(|s| Some(Box::new(s))),
                None => Ok(None),
            }?;
            let status = content.status.as_ref().map_or_else(
                || {
                    if schema.is_none() {
                        Ok(Some(HttpStatus::Code(204.try_into().unwrap())))
                    } else {
                        Ok(None)
                    }
                },
                |e| try_into_status(e.as_ref()).map(Some),
            )?;
            let media = content
                .media
                .as_ref()
                .map_or(Ok(None), |e| try_into_media(e.as_ref()).map(Some))?;
            let headers = content
                .headers
                .as_ref()
                .map_or(Ok(None), |h| Object::try_from(h.as_ref()).map(Some))?;
            let desc = e.annotation().and_then(|a| a.get_string("description"));
            Ok(Content {
                schema,
                status,
                media,
                headers,
                desc,
            })
        } else {
            Schema::try_from(e).map(Content::from)
        }
    }
}

pub type Ranges = IndexMap<(Option<HttpStatus>, Option<MediaType>), Content>;

fn try_into_ranges<T: AsExpr + Annotated>(ranges: &mut Ranges, e: &T) -> Result<()> {
    match e.as_node().as_expr() {
        ast::Expr::Op(op) if op.op == ast::Operator::Range => {
            op.exprs.iter().try_for_each(|r| try_into_ranges(ranges, r))
        }
        _ => {
            let c = Content::try_from(e)?;
            ranges.insert((c.status, c.media.clone()), c);
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transfer {
    pub methods: EnumMap<atom::Method, bool>,
    pub domain: Content,
    pub ranges: Ranges,
    pub params: Option<Object>,
    pub desc: Option<String>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub id: Option<String>,
}

impl Transfer {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        if let ast::Expr::Xfer(xfer) = e.as_node().as_expr() {
            let methods = xfer.methods;
            let mut ranges = IndexMap::new();
            try_into_ranges(&mut ranges, xfer.ranges.as_ref())?;
            let domain = match &xfer.domain {
                Some(d) => Content::try_from(d.as_ref()),
                None => Ok(Content::default()),
            }?;
            let params = match &xfer.params {
                Some(x) => Object::try_from(x.as_ref()).map(Some),
                None => Ok(None),
            }?;
            let ann = e.annotation();
            let desc = ann.and_then(|a| a.get_string("description"));
            let summary = ann.and_then(|a| a.get_string("summary"));
            let tags = ann.and_then(|a| a.get_enum("tags")).unwrap_or_default();
            let id = ann.and_then(|a| a.get_string("operationId"));
            Ok(Transfer {
                methods,
                domain,
                ranges,
                params,
                desc,
                summary,
                tags,
                id,
            })
        } else {
            Err(Error::new(Kind::UnexpectedExpression, "not a transfer").with(e))
        }
    }
}

pub type Transfers = EnumMap<atom::Method, Option<Transfer>>;

#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    pub uri: Uri,
    pub xfers: Transfers,
}

impl Relation {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        if let ast::Expr::Rel(rel) = e.as_node().as_expr() {
            let uri = Uri::try_from(rel.uri.as_ref())?;
            let mut xfers = Transfers::default();
            for x in rel.xfers.iter() {
                let t = Transfer::try_from(x)?;
                for (m, b) in t.methods {
                    if b {
                        xfers[m] = Some(t.clone());
                    }
                }
            }
            Ok(Relation { uri, xfers })
        } else {
            Err(Error::new(Kind::UnexpectedExpression, "not a relation").with(e))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Reference {
    Schema(Schema),
}

impl Reference {
    fn try_from<T: AsExpr + Annotated>(e: &T) -> Result<Self> {
        let s = Schema::try_from(e)?;
        Ok(Reference::Schema(s))
    }
}

pub type PathPattern = String;
pub type Relations = IndexMap<PathPattern, Relation>;
pub type References = IndexMap<Ident, Reference>;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Spec {
    pub rels: Relations,
    pub refs: References,
}

impl<T> TryFrom<&ModuleSet<T>> for Spec
where
    T: AsExpr + Annotated,
{
    type Error = Error;

    fn try_from(mods: &ModuleSet<T>) -> Result<Self> {
        let mut spec = Spec::default();
        let prg = mods.main();
        prg.scan(&mut spec, &mut Env::new(Some(mods)), &mut export)?;
        Ok(spec)
    }
}

/// Visits an abstract syntax tree to export references and relations.
fn export<T>(spec: &mut Spec, env: &mut Env<T>, node_ref: NodeRef<T>) -> Result<()>
where
    T: AsExpr + Annotated,
{
    match node_ref {
        NodeRef::Expr(expr) => {
            let node = expr.as_node();
            let span = node.span;
            match node.as_expr() {
                ast::Expr::Var(name) if name.is_reference() => match env.lookup(name) {
                    None => Err(Error::new(Kind::NotInScope, "").with(expr)),
                    Some(val) => {
                        let ref_ = Reference::try_from(val)?;
                        spec.refs.entry(name.clone()).or_insert(ref_);
                        Ok(())
                    }
                },
                _ => Ok(()),
            }
            .map_err(|err| err.at(span))
        }
        NodeRef::Res(res) => {
            let span = res.rel.as_node().span;
            let rel = Relation::try_from(&res.rel)?;
            match spec.rels.entry(rel.uri.pattern()) {
                indexmap::map::Entry::Vacant(v) => {
                    v.insert(rel);
                    Ok(())
                }
                indexmap::map::Entry::Occupied(_) => {
                    Err(Error::new(Kind::Conflict, "redefined relation").with(&rel))
                }
            }
            .map_err(|err| err.at(span))
        }
        _ => Ok(()),
    }
}
