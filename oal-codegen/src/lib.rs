mod oas;

use crate::oas::into_box_ref;
use indexmap::{indexmap, IndexMap};
use oal_compiler::spec;
use oal_syntax::atom::HttpStatusRange;
use oal_syntax::{ast, atom};
use openapiv3::{
    ArrayType, Components, Header, Info, IntegerType, MediaType, NumberType, ObjectType, OpenAPI,
    Operation, Parameter, ParameterData, ParameterSchemaOrContent, PathItem, Paths, ReferenceOr,
    RequestBody, Response, Responses, Schema, SchemaData, SchemaKind, Server, StatusCode,
    StringType, Type, VariantOrUnknownOrEmpty,
};
use std::iter::once;

#[derive(Default)]
pub struct Builder {
    spec: Option<spec::Spec>,
    base: Option<OpenAPI>,
}

type Headers = IndexMap<String, ReferenceOr<Header>>;

impl Builder {
    pub fn new() -> Builder {
        Builder::default()
    }

    pub fn with_base(mut self, base: OpenAPI) -> Self {
        self.base = Some(base);
        self
    }

    pub fn with_spec(mut self, spec: spec::Spec) -> Self {
        self.spec = Some(spec);
        self
    }

    pub fn into_openapi(self) -> OpenAPI {
        let paths = self.all_paths();
        let components = self.all_components();
        let mut definition = if let Some(base) = self.base {
            base
        } else {
            self.default_base()
        };
        definition.paths = paths;
        definition.components = Some(components);
        definition
    }

    fn default_base(&self) -> OpenAPI {
        OpenAPI {
            openapi: "3.0.3".into(),
            info: Info {
                title: "OpenAPI definition".into(),
                version: "0.1.0".into(),
                ..Default::default()
            },
            servers: vec![Server {
                url: "/".to_owned(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn media_type(&self) -> String {
        "application/json".to_owned()
    }

    fn uri_pattern(&self, uri: &spec::Uri) -> String {
        uri.pattern()
    }

    fn number_schema(&self, p: &spec::PrimNumber) -> Schema {
        let example = p.example.map(Into::into);
        Schema {
            schema_data: SchemaData {
                example,
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Number(NumberType {
                minimum: p.minimum,
                maximum: p.maximum,
                multiple_of: p.multiple_of,
                ..Default::default()
            })),
        }
    }

    fn string_schema(&self, p: &spec::PrimString) -> Schema {
        let example = p
            .example
            .clone()
            .or_else(|| p.enumeration.first().cloned())
            .map(Into::into);
        Schema {
            schema_data: SchemaData {
                example,
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::String(StringType {
                pattern: p.pattern.clone(),
                enumeration: p.enumeration.iter().map(|s| Some(s.clone())).collect(),
                ..Default::default()
            })),
        }
    }

    fn boolean_schema(&self, _: &spec::PrimBoolean) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Type(Type::Boolean {}),
        }
    }

    fn integer_schema(&self, p: &spec::PrimInteger) -> Schema {
        let example = p.example.map(Into::into);
        Schema {
            schema_data: SchemaData {
                example,
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Integer(IntegerType {
                minimum: p.minimum,
                maximum: p.maximum,
                multiple_of: p.multiple_of,
                ..Default::default()
            })),
        }
    }

    fn rel_schema(&self, rel: &spec::Relation) -> Schema {
        self.uri_schema(&rel.uri)
    }

    fn uri_schema(&self, uri: &spec::Uri) -> Schema {
        let example = uri
            .example
            .clone()
            .or_else(|| {
                if uri.path.is_empty() {
                    None
                } else {
                    Some(self.uri_pattern(uri))
                }
            })
            .map(Into::into);
        Schema {
            schema_data: SchemaData {
                example,
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::String(StringType {
                format: VariantOrUnknownOrEmpty::Unknown("uri-reference".into()),
                ..Default::default()
            })),
        }
    }

    fn join_schema(&self, schemas: &[spec::Schema]) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::AllOf {
                all_of: schemas.iter().map(|s| self.schema(s)).collect(),
            },
        }
    }

    fn object_type(&self, obj: &spec::Object) -> Type {
        let properties = obj
            .props
            .iter()
            .map(|p| {
                let ident = p.name.as_ref().into();
                let expr = into_box_ref(self.schema(&p.schema));
                (ident, expr)
            })
            .collect();
        let required = obj
            .props
            .iter()
            .filter_map(|p| {
                if p.schema.required.unwrap_or(false) {
                    Some(p.name.as_ref().to_owned())
                } else {
                    None
                }
            })
            .collect();
        Type::Object(ObjectType {
            properties,
            required,
            ..Default::default()
        })
    }

    fn object_schema(&self, obj: &spec::Object) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Type(self.object_type(obj)),
        }
    }

    fn array_schema(&self, array: &spec::Array) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Type(Type::Array(ArrayType {
                items: Some(into_box_ref(self.schema(&array.item))),
                min_items: None,
                max_items: None,
                unique_items: false,
            })),
        }
    }

    fn sum_schema(&self, schemas: &[spec::Schema]) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::OneOf {
                one_of: schemas.iter().map(|s| self.schema(s)).collect(),
            },
        }
    }

    fn any_schema(&self, schemas: &[spec::Schema]) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::AnyOf {
                any_of: schemas.iter().map(|s| self.schema(s)).collect(),
            },
        }
    }

    fn schema(&self, s: &spec::Schema) -> ReferenceOr<Schema> {
        if let spec::SchemaExpr::Ref(name) = &s.expr {
            ReferenceOr::Reference {
                reference: format!("#/components/schemas/{}", name.untagged()),
            }
        } else {
            let mut sch = match &s.expr {
                spec::SchemaExpr::Num(p) => self.number_schema(p),
                spec::SchemaExpr::Str(p) => self.string_schema(p),
                spec::SchemaExpr::Bool(p) => self.boolean_schema(p),
                spec::SchemaExpr::Int(p) => self.integer_schema(p),
                spec::SchemaExpr::Rel(rel) => self.rel_schema(rel),
                spec::SchemaExpr::Uri(uri) => self.uri_schema(uri),
                spec::SchemaExpr::Object(obj) => self.object_schema(obj),
                spec::SchemaExpr::Array(array) => self.array_schema(array),
                spec::SchemaExpr::Op(operation) => match operation.op {
                    ast::Operator::Join => self.join_schema(&operation.schemas),
                    ast::Operator::Sum => self.sum_schema(&operation.schemas),
                    ast::Operator::Any => self.any_schema(&operation.schemas),
                    ast::Operator::Range => unreachable!(),
                },
                spec::SchemaExpr::Ref(_) => unreachable!(),
            };
            sch.schema_data.description = s.desc.clone();
            sch.schema_data.title = s.title.clone();
            ReferenceOr::Item(sch)
        }
    }

    fn prop_param_data(&self, prop: &spec::Property, required: bool) -> ParameterData {
        ParameterData {
            name: prop.name.as_ref().into(),
            description: prop.desc.clone(),
            required,
            deprecated: None,
            format: ParameterSchemaOrContent::Schema(self.schema(&prop.schema)),
            example: None,
            examples: Default::default(),
            explode: None,
            extensions: Default::default(),
        }
    }

    fn prop_path_param(&self, prop: &spec::Property) -> Parameter {
        Parameter::Path {
            parameter_data: self.prop_param_data(prop, true),
            style: Default::default(),
        }
    }

    fn prop_query_param(&self, prop: &spec::Property) -> Parameter {
        Parameter::Query {
            parameter_data: self.prop_param_data(prop, prop.required.unwrap_or(false)),
            allow_reserved: false,
            style: Default::default(),
            allow_empty_value: None,
        }
    }

    fn prop_header_param(&self, prop: &spec::Property) -> Parameter {
        Parameter::Header {
            parameter_data: self.prop_param_data(prop, prop.required.unwrap_or(false)),
            style: Default::default(),
        }
    }

    fn prop_header(&self, prop: &spec::Property) -> Header {
        Header {
            description: prop.desc.clone(),
            style: Default::default(),
            required: prop.required.unwrap_or(false),
            deprecated: None,
            format: ParameterSchemaOrContent::Schema(self.schema(&prop.schema)),
            example: None,
            examples: Default::default(),
            extensions: Default::default(),
        }
    }

    fn xfer_params(&self, xfer: &spec::Transfer) -> Vec<ReferenceOr<Parameter>> {
        xfer.params
            .iter()
            .flat_map(|o| {
                o.props
                    .iter()
                    .map(|p| ReferenceOr::Item(self.prop_query_param(p)))
            })
            .chain(xfer.domain.headers.iter().flat_map(|o| {
                o.props
                    .iter()
                    .map(|p| ReferenceOr::Item(self.prop_header_param(p)))
            }))
            .collect()
    }

    fn uri_params(&self, uri: &spec::Uri) -> Vec<ReferenceOr<Parameter>> {
        let path = uri.path.iter().flat_map(|s| match s {
            spec::UriSegment::Variable(p) => Some(ReferenceOr::Item(self.prop_path_param(p))),
            _ => None,
        });
        let query = uri.params.iter().flat_map(|o| {
            o.props
                .iter()
                .map(|p| ReferenceOr::Item(self.prop_query_param(p)))
        });
        path.chain(query).collect()
    }

    fn domain_request(&self, domain: &spec::Content) -> Option<ReferenceOr<RequestBody>> {
        let media = domain.media.clone().unwrap_or_else(|| self.media_type());
        domain.schema.as_ref().map(|schema| {
            ReferenceOr::Item(RequestBody {
                content: indexmap! { media => MediaType {
                    schema: Some(self.schema(schema)),
                    ..Default::default()
                }},
                description: domain.desc.clone(),
                ..Default::default()
            })
        })
    }

    fn xfer_request(&self, xfer: &spec::Transfer) -> Option<ReferenceOr<RequestBody>> {
        self.domain_request(&xfer.domain)
    }

    fn http_status_code(&self, status: &atom::HttpStatus) -> StatusCode {
        match *status {
            atom::HttpStatus::Code(code) => StatusCode::Code(code.into()),
            atom::HttpStatus::Range(range) => StatusCode::Range(match range {
                HttpStatusRange::Info => 1,
                HttpStatusRange::Success => 2,
                HttpStatusRange::Redirect => 3,
                HttpStatusRange::ClientError => 4,
                HttpStatusRange::ServerError => 5,
            }),
        }
    }

    fn content_headers(&self, content: &spec::Content) -> Headers {
        content.headers.as_ref().map_or_else(Headers::default, |h| {
            h.props
                .iter()
                .map(|p| {
                    (
                        p.name.as_ref().to_owned(),
                        ReferenceOr::Item(self.prop_header(p)),
                    )
                })
                .collect()
        })
    }

    fn xfer_responses(&self, xfer: &spec::Transfer) -> Responses {
        let mut default = None;
        let mut responses = IndexMap::new();

        for ((status, media), content) in xfer.ranges.iter() {
            let response = if let Some(s) = status {
                responses
                    .entry(self.http_status_code(s))
                    .or_insert(ReferenceOr::Item(Response::default()))
            } else {
                default.insert(ReferenceOr::Item(Response::default()))
            };
            if let ReferenceOr::Item(res) = response {
                if let Some(schema) = content.schema.as_ref() {
                    let media_type = media.clone().unwrap_or_else(|| self.media_type());
                    let media_schema = MediaType {
                        schema: Some(self.schema(schema)),
                        ..Default::default()
                    };
                    res.content.insert(media_type, media_schema);
                }
                res.headers = self.content_headers(content);
                res.description = content.desc.clone().unwrap_or_else(|| "".to_owned());
            } else {
                unreachable!();
            }
        }

        Responses {
            default,
            responses,
            ..Default::default()
        }
    }

    fn method_label(&self, m: atom::Method) -> &str {
        match m {
            atom::Method::Get => "get",
            atom::Method::Put => "put",
            atom::Method::Post => "post",
            atom::Method::Patch => "patch",
            atom::Method::Delete => "delete",
            atom::Method::Options => "options",
            atom::Method::Head => "head",
        }
    }

    fn uri_segment_label(&self, s: &spec::UriSegment) -> String {
        match s {
            spec::UriSegment::Literal(l) => {
                if l.is_empty() {
                    "root".to_owned()
                } else {
                    l.to_lowercase()
                }
            }
            spec::UriSegment::Variable(t) => t.name.as_ref().to_lowercase(),
        }
    }

    fn xfer_id(
        &self,
        xfer: &spec::Transfer,
        method: atom::Method,
        uri: &spec::Uri,
    ) -> Option<String> {
        if xfer.id.is_some() {
            return xfer.id.clone();
        }
        let prefix = self.method_label(method).to_owned();
        let label = once(prefix)
            .chain(uri.path.iter().map(|s| self.uri_segment_label(s)))
            .collect::<Vec<_>>()
            .join("-");
        Some(label)
    }

    fn relation_path_item(&self, rel: &spec::Relation) -> PathItem {
        let mut path_item = PathItem {
            parameters: self.uri_params(&rel.uri),
            ..Default::default()
        };

        let xfers = rel
            .xfers
            .iter()
            .filter_map(|(m, x)| x.as_ref().map(|x| (m, x)));

        for (method, xfer) in xfers {
            let operation_id = self.xfer_id(xfer, method, &rel.uri);
            let summary = xfer
                .summary
                .clone()
                .or_else(|| xfer.desc.clone())
                .or_else(|| operation_id.clone());
            let description = xfer.desc.clone();

            let op = Operation {
                summary,
                description,
                operation_id,
                parameters: self.xfer_params(xfer),
                request_body: self.xfer_request(xfer),
                responses: self.xfer_responses(xfer),
                tags: xfer.tags.clone(),
                ..Default::default()
            };

            match method {
                atom::Method::Get => path_item.get = Some(op),
                atom::Method::Put => path_item.put = Some(op),
                atom::Method::Post => path_item.post = Some(op),
                atom::Method::Patch => path_item.patch = Some(op),
                atom::Method::Delete => path_item.delete = Some(op),
                atom::Method::Options => path_item.options = Some(op),
                atom::Method::Head => path_item.head = Some(op),
            }
        }

        path_item
    }

    fn all_paths(&self) -> Paths {
        let paths = if let Some(spec) = &self.spec {
            spec.rels
                .iter()
                .map(|(pattern, rel)| {
                    (
                        pattern.clone(),
                        ReferenceOr::Item(self.relation_path_item(rel)),
                    )
                })
                .collect()
        } else {
            Default::default()
        };
        Paths {
            paths,
            extensions: Default::default(),
        }
    }

    fn all_components(&self) -> Components {
        let schemas = if let Some(spec) = &self.spec {
            spec.refs
                .iter()
                .flat_map(|(name, reference)| match reference {
                    spec::Reference::Schema(s) => Some((name.untagged(), self.schema(s))),
                })
                .collect()
        } else {
            Default::default()
        };
        Components {
            schemas,
            ..Default::default()
        }
    }
}

impl From<Builder> for OpenAPI {
    fn from(b: Builder) -> Self {
        b.into_openapi()
    }
}
