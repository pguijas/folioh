//! The IR structs and enums; field order is the JSON key order.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

/// The parser that produced a module; routes and symbol keys derive from it.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Python,
    Javascript,
    Rust,
}

impl Language {
    /// The serialised id: `python`, `javascript`, `rust`.
    pub fn id(self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::Javascript => "javascript",
            Language::Rust => "rust",
        }
    }

    /// The heading label: `Python`, `JavaScript`, `Rust`.
    pub fn label(self) -> &'static str {
        match self {
            Language::Python => "Python",
            Language::Javascript => "JavaScript",
            Language::Rust => "Rust",
        }
    }

    /// The language for an id, `None` for anything outside the closed set.
    pub fn parse(id: &str) -> Option<Language> {
        match id {
            "python" => Some(Language::Python),
            "javascript" => Some(Language::Javascript),
            "rust" => Some(Language::Rust),
            _ => None,
        }
    }
}

/// Where an argument sits in a Python signature.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArgKind {
    #[default]
    Regular,
    PositionalOnly,
    KeywordOnly,
    VarPositional,
    VarKeyword,
}

/// What a function is: a free function or one of the method flavours.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum FunctionKind {
    #[default]
    Function,
    Method,
    Property,
    Staticmethod,
    Classmethod,
}

/// The closed set of non-class type items other languages publish.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    Struct,
    Enum,
    Trait,
    Interface,
    TypeAlias,
    Union,
    Impl,
}

/// A `TypeIR.kind` outside the closed set; the payload is the offending value.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InvalidTypeKind(pub String);

impl fmt::Display for InvalidTypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TypeIR.kind must be one of ['enum', 'impl', 'interface', 'struct', 'trait', 'type_alias', 'union'], got '{}'",
            self.0
        )
    }
}

impl std::error::Error for InvalidTypeKind {}

impl TypeKind {
    /// The heading label per kind (`type_alias` reads `Type`).
    pub fn label(self) -> &'static str {
        match self {
            TypeKind::Struct => "Struct",
            TypeKind::Enum => "Enum",
            TypeKind::Trait => "Trait",
            TypeKind::Interface => "Interface",
            TypeKind::TypeAlias => "Type",
            TypeKind::Union => "Union",
            TypeKind::Impl => "Impl",
        }
    }

    /// The kind for an id; anything else is `InvalidTypeKind`.
    pub fn parse(s: &str) -> Result<TypeKind, InvalidTypeKind> {
        match s {
            "struct" => Ok(TypeKind::Struct),
            "enum" => Ok(TypeKind::Enum),
            "trait" => Ok(TypeKind::Trait),
            "interface" => Ok(TypeKind::Interface),
            "type_alias" => Ok(TypeKind::TypeAlias),
            "union" => Ok(TypeKind::Union),
            "impl" => Ok(TypeKind::Impl),
            other => Err(InvalidTypeKind(other.to_string())),
        }
    }
}

impl<'de> Deserialize<'de> for TypeKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        TypeKind::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// One function parameter.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ArgIR {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub default: Option<String>,
    pub description: String,
    #[serde(default)]
    pub kind: ArgKind,
}

/// A documented return value or yielded item.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct ReturnIR {
    #[serde(rename = "type")]
    pub ty: String,
    pub description: String,
    /// `type` describes each yielded item rather than the function's return.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_generator: bool,
    /// The original Python return annotation for a yielding section's signature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_annotation: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// One documented exception.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RaiseIR {
    pub exception: String,
    pub description: String,
}

/// The `type` of a constant that is a re-export (Rust `pub use`): its `name`
/// is the statement as written, and it renders as a re-export, not a value.
pub const REEXPORT: &str = "re-export";

/// A constant, class variable, field or enum variant.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct VarIR {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub value: String,
    pub description: String,
}

/// The parsed prose of a docstring.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct DocstringIR {
    pub short_description: String,
    #[serde(default)]
    pub long_description: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// A function or method.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct FunctionIR {
    pub name: String,
    pub args: Vec<ArgIR>,
    pub returns: Option<ReturnIR>,
    pub raises: Vec<RaiseIR>,
    pub decorators: Vec<String>,
    pub docstring: DocstringIR,
    pub is_async: bool,
    pub source_file: String,
    pub line_number: u32,
    #[serde(default)]
    pub kind: FunctionKind,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub visibility: String,
}

/// A Python class, recursively.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ClassIR {
    pub name: String,
    pub bases: Vec<String>,
    pub decorators: Vec<String>,
    pub docstring: DocstringIR,
    pub methods: Vec<FunctionIR>,
    pub class_vars: Vec<VarIR>,
    #[serde(default)]
    pub inner_classes: Vec<ClassIR>,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub line_number: u32,
}

/// A named type item that is not a Python class.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TypeIR {
    pub name: String,
    pub kind: TypeKind,
    pub docstring: DocstringIR,
    #[serde(default)]
    pub fields: Vec<VarIR>,
    #[serde(default)]
    pub variants: Vec<VarIR>,
    #[serde(default)]
    pub methods: Vec<FunctionIR>,
    #[serde(default)]
    pub bases: Vec<String>,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub visibility: String,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub line_number: u32,
}

/// One parsed source module.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ModuleIR {
    pub name: String,
    pub docstring: DocstringIR,
    pub classes: Vec<ClassIR>,
    pub functions: Vec<FunctionIR>,
    pub constants: Vec<VarIR>,
    pub source_file: String,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub types: Vec<TypeIR>,
}

#[cfg(test)]
#[path = "ir_tests.rs"]
mod tests;
