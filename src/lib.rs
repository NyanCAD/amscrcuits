use std::collections::HashMap;
use std::path::{Path, PathBuf};
use handlebars::Handlebars;
use serde::Serialize;

pub struct Entity {
    pub name: String,
    pub generic: Vec<String>,
    pub port: Vec<String>,
    pub archs: HashMap<String, Arch>,
}

pub enum Arch {
    Symbol(Symbol),
    Schematic(Schematic),
    Code(CodeDialectArch),
    //TranspiledCode(???),
}

pub struct Symbol;
pub struct Schematic;

#[derive(Debug)]
pub enum CodeError {
    DialectError,
    CompileError,
    TemplateError(handlebars::TemplateRenderError),
}

impl From<handlebars::TemplateRenderError> for CodeError {
    fn from(error: handlebars::TemplateRenderError) -> Self {
        CodeError::TemplateError(error)
    }
}

#[derive(Serialize)]
struct RefArgs {
  generic: HashMap<String, String>,
  port: HashMap<String, String>,
}

pub trait Code {
    fn definition(&self) -> Result<&Path, CodeError> { Err(CodeError::DialectError) }
    fn reference(&self, _genericmap: HashMap<String, String>, _portmap: HashMap<String, String>) -> Result<String, CodeError> { Err(CodeError::DialectError) }
}

/// Contains a definition in some language
/// and a Handlebars template for referencing the definition
struct CodeArch {
    definition: PathBuf,
    reference: String,
}

impl Code for CodeArch {
    fn definition(&self) -> Result<&Path, CodeError> { Ok(self.definition.as_path()) }
    fn reference(&self, genericmap: HashMap<String, String>, portmap: HashMap<String, String>) -> Result<String, CodeError> {
        let handlebars = Handlebars::new();
        let varmap = RefArgs {generic: genericmap, port: portmap};
        let reference = handlebars.render_template(&self.reference, &varmap)?;
        Ok(reference)
    }
}

/// Contains multiple dialectso of a given subcircuit/model
/// Maps from a spice dialect to a definition
pub struct CodeDialectArch {
    dialects: HashMap<String, CodeArch>,
}

impl CodeDialectArch {
    pub fn new() -> CodeDialectArch {
        CodeDialectArch {dialects: HashMap::new()}
    }
}

pub struct Ngspice<T>(T);

pub struct Xyce<T>(T);

pub struct Verilator<T>(T);

// CXXRTL takes anything Yosys can read plus C++
// pub struct CXXRTL<T>(T);

pub struct GHDL<T>(T);

// nMigen takes Python files
// pub struct NMigen<T>(T);

impl Code for Ngspice<&CodeDialectArch> {
    fn definition(&self) -> Result<&Path, CodeError> {
        let arch = self.0.dialects.get("ngspice")
            .or_else(|| self.0.dialects.get("spice"))
            .ok_or(CodeError::DialectError)?;
        arch.definition()
    }
    fn reference(&self, genericmap: HashMap<String, String>, portmap: HashMap<String, String>) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("ngspice")
            .or_else(|| self.0.dialects.get("spice"))
            .ok_or(CodeError::DialectError)?;
        arch.reference(genericmap, portmap)
    }
}

impl Code for Xyce<&CodeDialectArch> {
    fn definition(&self) -> Result<&Path, CodeError> {
        let arch = self.0.dialects.get("xyce")
            .or_else(|| self.0.dialects.get("spice"))
            .ok_or(CodeError::DialectError)?;
        arch.definition()
    }
    fn reference(&self, genericmap: HashMap<String, String>, portmap: HashMap<String, String>) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("xyce")
            .or_else(|| self.0.dialects.get("spice"))
            .ok_or(CodeError::DialectError)?;
        arch.reference(genericmap, portmap)
    }
}

impl Code for GHDL<&CodeDialectArch> {
    fn definition(&self) -> Result<&Path, CodeError> {
        let arch = self.0.dialects.get("ghdl")
            .or_else(|| self.0.dialects.get("vhdl"))
            .ok_or(CodeError::DialectError)?;
        arch.definition()
    }
    fn reference(&self, genericmap: HashMap<String, String>, portmap: HashMap<String, String>) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("ghdl")
            .or_else(|| self.0.dialects.get("vhdl"))
            .ok_or(CodeError::DialectError)?;
        arch.reference(genericmap, portmap)
    }
}

impl Code for Verilator<&CodeDialectArch> {
    fn definition(&self) -> Result<&Path, CodeError> {
        let arch = self.0.dialects.get("verilator")
            .or_else(|| self.0.dialects.get("verilog"))
            .ok_or(CodeError::DialectError)?;
        arch.definition()
    }
    fn reference(&self, genericmap: HashMap<String, String>, portmap: HashMap<String, String>) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("verilator")
            .or_else(|| self.0.dialects.get("verilog"))
            .ok_or(CodeError::DialectError)?;
        arch.reference(genericmap, portmap)
    }
}

#[cfg(test)]
mod tests {
    use vhdl_lang::{VHDLParser, Diagnostic, ast};
    use std::path::Path;
    use super::*;

    #[test]
    fn code_arch() {
        let code = CodeArch { reference: "{{generic.name}}, {{port.platitude}}".to_string(), definition: PathBuf::from("hello")};
        let mut generics = HashMap::new();
        generics.insert("name".to_string(), "world".to_string());
        let mut ports = HashMap::new();
        ports.insert("platitude".to_string(), "whatsup".to_string());
        assert_eq!(code.definition().unwrap(), Path::new("hello"));
        assert_eq!(code.reference(generics, ports).unwrap(), "world, whatsup");
    }

    #[test]
    fn spice_arch() {
        let mut spice = CodeDialectArch::new();
        spice.dialects.insert("spice".into(), CodeArch {definition: "this is spice".into(), reference: "spice ref".into()});
        spice.dialects.insert("ngspice".into(), CodeArch {definition: "this is ngspice".into(), reference: "ngspice ref".into()});
        assert_eq!(Ngspice(&spice).definition().unwrap(), Path::new("this is ngspice"));
        assert_eq!(Xyce(&spice).definition().unwrap(), Path::new("this is spice"));
        assert_eq!(Ngspice(&spice).reference(HashMap::new(), HashMap::new()).unwrap(), "ngspice ref");
        assert_eq!(Xyce(&spice).reference(HashMap::new(), HashMap::new()).unwrap(), "spice ref");


        let mut spice = CodeDialectArch::new();
        spice.dialects.insert("spice".into(), CodeArch {definition: "this is spice".into(), reference: "spice ref".into()});
        spice.dialects.insert("xyce".into(), CodeArch {definition: "this is xyce".into(), reference: "xyce ref".into()});
        assert_eq!(Ngspice(&spice).definition().unwrap(), Path::new("this is spice"));
        assert_eq!(Xyce(&spice).definition().unwrap(), Path::new("this is xyce"));
        assert_eq!(Ngspice(&spice).reference(HashMap::new(), HashMap::new()).unwrap(), "spice ref");
        assert_eq!(Xyce(&spice).reference(HashMap::new(), HashMap::new()).unwrap(), "xyce ref");
    }

    #[test]
    fn verilator_arch() {
        let mut verilog = CodeDialectArch::new();
        verilog.dialects.insert("verilog".into(), CodeArch {definition: "this is verilog".into(), reference: "verilog ref".into()});
        assert_eq!(Verilator(&verilog).definition().unwrap(), Path::new("this is verilog"));
        assert_eq!(Verilator(&verilog).reference(HashMap::new(), HashMap::new()).unwrap(), "verilog ref");

        verilog.dialects.insert("verilator".into(), CodeArch {definition: "this is verilator".into(), reference: "verilator ref".into()});
        assert_eq!(Verilator(&verilog).definition().unwrap(), Path::new("this is verilator"));
        assert_eq!(Verilator(&verilog).reference(HashMap::new(), HashMap::new()).unwrap(), "verilator ref");
        assert_eq!(Ngspice(&verilog).definition().is_err(), true);
    }

    #[test]
    fn ghdl_arch() {
        let mut vhdl = CodeDialectArch::new();
        vhdl.dialects.insert("vhdl".into(), CodeArch {definition: "this is vhdl".into(), reference: "vhdl ref".into()});
        assert_eq!(GHDL(&vhdl).definition().unwrap(), Path::new("this is vhdl"));
        assert_eq!(GHDL(&vhdl).reference(HashMap::new(), HashMap::new()).unwrap(), "vhdl ref");

        vhdl.dialects.insert("ghdl".into(), CodeArch {definition: "this is ghdl".into(), reference: "ghdl ref".into()});
        assert_eq!(GHDL(&vhdl).definition().unwrap(), Path::new("this is ghdl"));
        assert_eq!(GHDL(&vhdl).reference(HashMap::new(), HashMap::new()).unwrap(), "ghdl ref");
        assert_eq!(Xyce(&vhdl).definition().is_err(), true);
    }

    #[test]
    fn parse_ent() {
        let mut diag: Vec<Diagnostic> = Vec::new();
        let parser = VHDLParser::default();
        let (_source, file) = parser.parse_design_file(Path::new("data/ent.vhdl"), &mut diag).unwrap();
        if let ast::AnyDesignUnit::Primary(ast::AnyPrimaryUnit::Entity(entity)) = &file.design_units[0] {
            let name = entity.ident.item.name_utf8();
            assert_eq!(name, "PARITY");
            for gen in entity.generic_clause.as_ref().unwrap_or(&Vec::new()) {
                if let ast::InterfaceDeclaration::Object(obj) = gen {
                    let name = obj.ident.item.name_utf8();
                    assert_eq!(name, "N");
                }
            }
        }
        println!("{:#?}", file);
    }
}
