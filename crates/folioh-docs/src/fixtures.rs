use folioh_ir::{
    ArgIR, ArgKind, ClassIR, DocstringIR, FunctionIR, FunctionKind, Language, ModuleIR, ReturnIR,
    TypeIR, TypeKind, VarIR,
};

pub fn doc(short: &str) -> DocstringIR {
    DocstringIR {
        short_description: short.to_string(),
        ..DocstringIR::default()
    }
}

pub fn arg(name: &str, ty: &str, default: Option<&str>, description: &str) -> ArgIR {
    ArgIR {
        name: name.to_string(),
        ty: ty.to_string(),
        default: default.map(str::to_string),
        description: description.to_string(),
        kind: ArgKind::Regular,
    }
}

pub fn ret(ty: &str, description: &str) -> ReturnIR {
    ReturnIR {
        ty: ty.to_string(),
        description: description.to_string(),
        ..ReturnIR::default()
    }
}

pub fn var(name: &str, ty: &str, description: &str) -> VarIR {
    VarIR {
        name: name.to_string(),
        ty: ty.to_string(),
        value: String::new(),
        description: description.to_string(),
    }
}

pub fn function(name: &str, args: Vec<ArgIR>, returns: Option<ReturnIR>) -> FunctionIR {
    FunctionIR {
        name: name.to_string(),
        args,
        returns,
        raises: vec![],
        decorators: vec![],
        docstring: DocstringIR::default(),
        is_async: false,
        source_file: "test.py".to_string(),
        line_number: 1,
        kind: FunctionKind::Function,
        signature: String::new(),
        visibility: String::new(),
    }
}

pub fn class(name: &str, bases: &[&str], methods: Vec<FunctionIR>) -> ClassIR {
    ClassIR {
        name: name.to_string(),
        bases: bases.iter().map(|b| b.to_string()).collect(),
        decorators: vec![],
        docstring: DocstringIR::default(),
        methods,
        class_vars: vec![],
        inner_classes: vec![],
        source_file: "test.py".to_string(),
        line_number: 1,
    }
}

pub fn type_item(name: &str, kind: TypeKind) -> TypeIR {
    TypeIR {
        name: name.to_string(),
        kind,
        docstring: DocstringIR::default(),
        fields: vec![],
        variants: vec![],
        methods: vec![],
        bases: vec![],
        signature: String::new(),
        visibility: String::new(),
        source_file: String::new(),
        line_number: 0,
    }
}

pub fn module(name: &str, classes: Vec<ClassIR>, functions: Vec<FunctionIR>) -> ModuleIR {
    ModuleIR {
        name: name.to_string(),
        docstring: DocstringIR::default(),
        classes,
        functions,
        constants: vec![],
        source_file: format!("{}.py", name.replace('.', "/")),
        language: Language::Python,
        types: vec![],
    }
}
