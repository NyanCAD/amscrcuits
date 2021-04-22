use std::collections::HashMap;
use std::rc::Rc;
use handlebars::Handlebars;
use serde::Serialize;
// use petgraph::graph::{Graph, UnGraph, NodeIndex};

pub struct Entity {
    pub name: String,
    pub symbol: Symbol,
    pub generic: Vec<String>,
    pub port: Vec<String>,
    pub archs: HashMap<String, Arch>,
}

pub enum Arch {
    Schematic(Schematic),
    Code(CodeDialectArch),
    //TranspiledCode(???),
}

pub struct Symbol;

pub struct Configuration;

// TODO instances and schematics require a complete rework for GUI interface
pub struct Instance {
    pub portmap: HashMap<String, String>,
    pub genericmap: HashMap<String, String>,
    pub x: i64,
    pub y: i64,
    pub entity: Rc<Entity>,
}

pub struct Schematic {
    pub instances: HashMap<String, Instance>,
}

// TODO rewrite with configurations
impl Code for Ngspice<&Schematic> {
    fn definition(&self) -> Result<String, CodeError> {
        let sch = self.0;
        let mut code = String::new();
        // Don't repeat models... globaly!
        // Do at the configuration level?
        for (_name, inst) in &sch.instances {
            match &inst.entity.archs["rtl"] { // configuration!!
                Arch::Code(arch) => code.push_str(&Ngspice(arch).definition()?),
                Arch::Schematic(arch) => code.push_str(&Ngspice(arch).definition()?)
            }
           code.push_str("\n");
        }
        for (name, inst) in &sch.instances {
            match &inst.entity.archs["rtl"] { // configuration!!
                Arch::Code(arch) => code.push_str(&Ngspice(arch).reference(name, &inst.genericmap, &inst.portmap)?),
                Arch::Schematic(arch) => code.push_str(&Ngspice(arch).definition()?)
            }
           code.push_str("\n");
        }
        Ok(code)
    }
    fn reference(&self, _name: &str, _genericmap: &HashMap<String, String>, _portmap: &HashMap<String, String>) -> Result<String, CodeError> {
        Err(CodeError::DialectError)
    }
}

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
struct RefArgs<'a> {
    name: &'a str,
    generic: &'a HashMap<String, String>,
    port: &'a HashMap<String, String>,
}

pub trait Code {
    fn definition(&self) -> Result<String, CodeError> { Err(CodeError::DialectError) }
    fn reference(&self, _name: &str, _genericmap: &HashMap<String, String>, _portmap: &HashMap<String, String>) -> Result<String, CodeError> { Err(CodeError::DialectError) }
}

/// Contains a definition in some language
/// and a Handlebars template for referencing the definition
struct CodeArch {
    definition: String,
    reference: String,
}

impl Code for CodeArch {
    fn definition(&self) -> Result<String, CodeError> { Ok(self.definition.clone()) }
    fn reference(&self, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError> {
        let handlebars = Handlebars::new();
        let varmap = RefArgs {name: name, generic: genericmap, port: portmap};
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
    fn definition(&self) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("ngspice")
            .or_else(|| self.0.dialects.get("spice"))
            .ok_or(CodeError::DialectError)?;
        arch.definition()
    }
    fn reference(&self, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("ngspice")
            .or_else(|| self.0.dialects.get("spice"))
            .ok_or(CodeError::DialectError)?;
        arch.reference(name, genericmap, portmap)
    }
}

impl Code for Xyce<&CodeDialectArch> {
    fn definition(&self) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("xyce")
            .or_else(|| self.0.dialects.get("spice"))
            .ok_or(CodeError::DialectError)?;
        arch.definition()
    }
    fn reference(&self, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("xyce")
            .or_else(|| self.0.dialects.get("spice"))
            .ok_or(CodeError::DialectError)?;
        arch.reference(name, genericmap, portmap)
    }
}

impl Code for GHDL<&CodeDialectArch> {
    fn definition(&self) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("ghdl")
            .or_else(|| self.0.dialects.get("vhdl"))
            .ok_or(CodeError::DialectError)?;
        arch.definition()
    }
    fn reference(&self, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("ghdl")
            .or_else(|| self.0.dialects.get("vhdl"))
            .ok_or(CodeError::DialectError)?;
        arch.reference(name, genericmap, portmap)
    }
}

impl Code for Verilator<&CodeDialectArch> {
    fn definition(&self) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("verilator")
            .or_else(|| self.0.dialects.get("verilog"))
            .ok_or(CodeError::DialectError)?;
        arch.definition()
    }
    fn reference(&self, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError> {
        let arch = self.0.dialects.get("verilator")
            .or_else(|| self.0.dialects.get("verilog"))
            .ok_or(CodeError::DialectError)?;
        arch.reference(name, genericmap, portmap)
    }
}

#[cfg(test)]
mod tests {
    use vhdl_lang::{VHDLParser, Diagnostic, ast};
    use std::path::Path;
    use super::*;

    /// Macro for HashMap literals
    macro_rules! collection {
        // map-like
        ($($k:expr => $v:expr),* $(,)?) => {
            std::iter::Iterator::collect(std::array::IntoIter::new([$(($k, $v),)*]))
        };
        // set-like
        ($($v:expr),* $(,)?) => {
            std::iter::Iterator::collect(std::array::IntoIter::new([$($v,)*]))
        };
    }


    #[test]
    fn circuit() {
        let code = CodeArch {
            reference: "m{{name}} {{port.d}} {{port.g}} {{port.s}} {{port.b}} PMOS W={{generic.w}} L={{generic.l}}".into(),
            definition: ".model PMOS".into()
        };
        let mut spicemos = CodeDialectArch::new();
        spicemos.dialects.insert("spice".into(), code);
        let mut arches = HashMap::new();
        arches.insert("rtl".into(), Arch::Code(spicemos));

        let pmos = Rc::from(Entity {
            name: "pmos".into(),
            symbol: Symbol {},
            generic: vec!["w".into(), "l".into()],
            port: vec!["g".into(), "d".into(), "s".into(), "b".into()],
            archs: arches,
        });

        let code = CodeArch {
            reference: "m{{name}} {{port.d}} {{port.g}} {{port.s}} {{port.b}} NMOS W={{generic.w}} L={{generic.l}}".to_string(),
            definition: ".model NMOS".into()
        };
        let mut spicemos = CodeDialectArch::new();
        spicemos.dialects.insert("spice".into(), code);
        let mut arches = HashMap::new();
        arches.insert("rtl".into(), Arch::Code(spicemos));

        let nmos = Rc::from(Entity {
            name: "nmos".into(),
            symbol: Symbol {},
            generic: vec!["w".into(), "l".into()],
            port: vec!["g".into(), "d".into(), "s".into(), "b".into()],
            archs: arches,
        });

        let mut cir = Schematic {
            instances: HashMap::new(),
        };
        cir.instances.insert(
                "pmos1".into(),
                Instance {
                    genericmap: HashMap::new(),
                    portmap: collection!{
                        "g".into() => "in".into(),
                        "d".into() => "mid".into(),
                        "s".into() => "vdd".into(),
                        "b".into() => "vdd".into(),
                    },
                    x: 0,
                    y: 0,
                    entity: pmos.clone(),
                });
        cir.instances.insert(
                "nmos1".into(),
                Instance {
                    genericmap: HashMap::new(),
                    portmap: collection!{
                        "g".into() => "in".into(),
                        "d".into() => "mid".into(),
                        "s".into() => "vss".into(),
                        "b".into() => "vss".into(),
                    },
                    x: 0,
                    y: 0,
                    entity: nmos.clone(),
                });
        cir.instances.insert(
                "pmos2".into(),
                Instance {
                    genericmap: HashMap::new(),
                    portmap: collection!{
                        "g".into() => "mid".into(),
                        "d".into() => "out".into(),
                        "s".into() => "vdd".into(),
                        "b".into() => "vdd".into(),
                    },
                    x: 0,
                    y: 0,
                    entity: pmos.clone(),
                });

        cir.instances.insert(
                "nmos2".into(),
                Instance {
                    genericmap: HashMap::new(),
                    portmap: collection!{
                        "g".into() => "mid".into(),
                        "d".into() => "out".into(),
                        "s".into() => "vss".into(),
                        "b".into() => "vss".into(),
                    },
                    x: 0,
                    y: 0,
                    entity: nmos.clone(),
                });
        println!("{}", Ngspice(&cir).definition().unwrap());
        assert_eq!(Ngspice(&cir).definition().unwrap(), "");
    }

    #[test]
    fn code_arch() {
        let code = CodeArch { reference: "{{generic.name}}, {{port.platitude}}".to_string(), definition: "hello".into()};
        let mut generics = HashMap::new();
        generics.insert("name".to_string(), "world".to_string());
        let mut ports = HashMap::new();
        ports.insert("platitude".to_string(), "whatsup".to_string());
        assert_eq!(code.definition().unwrap(), "hello");
        assert_eq!(code.reference("foo", &generics, &ports).unwrap(), "world, whatsup");
    }

    #[test]
    fn spice_arch() {
        let mut spice = CodeDialectArch::new();
        spice.dialects.insert("spice".into(), CodeArch {definition: "this is spice".into(), reference: "spice ref".into()});
        spice.dialects.insert("ngspice".into(), CodeArch {definition: "this is ngspice".into(), reference: "ngspice ref".into()});
        assert_eq!(Ngspice(&spice).definition().unwrap(), "this is ngspice");
        assert_eq!(Xyce(&spice).definition().unwrap(), "this is spice");
        assert_eq!(Ngspice(&spice).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "ngspice ref");
        assert_eq!(Xyce(&spice).reference("bar", &HashMap::new(), &HashMap::new()).unwrap(), "spice ref");


        let mut spice = CodeDialectArch::new();
        spice.dialects.insert("spice".into(), CodeArch {definition: "this is spice".into(), reference: "spice ref".into()});
        spice.dialects.insert("xyce".into(), CodeArch {definition: "this is xyce".into(), reference: "xyce ref".into()});
        assert_eq!(Ngspice(&spice).definition().unwrap(), "this is spice");
        assert_eq!(Xyce(&spice).definition().unwrap(), "this is xyce");
        assert_eq!(Ngspice(&spice).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "spice ref");
        assert_eq!(Xyce(&spice).reference("bar", &HashMap::new(), &HashMap::new()).unwrap(), "xyce ref");
    }

    #[test]
    fn verilator_arch() {
        let mut verilog = CodeDialectArch::new();
        verilog.dialects.insert("verilog".into(), CodeArch {definition: "this is verilog".into(), reference: "verilog ref".into()});
        assert_eq!(Verilator(&verilog).definition().unwrap(), "this is verilog");
        assert_eq!(Verilator(&verilog).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "verilog ref");

        verilog.dialects.insert("verilator".into(), CodeArch {definition: "this is verilator".into(), reference: "verilator ref".into()});
        assert_eq!(Verilator(&verilog).definition().unwrap(), "this is verilator");
        assert_eq!(Verilator(&verilog).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "verilator ref");
        assert_eq!(Ngspice(&verilog).definition().is_err(), true);
    }

    #[test]
    fn ghdl_arch() {
        let mut vhdl = CodeDialectArch::new();
        vhdl.dialects.insert("vhdl".into(), CodeArch {definition: "this is vhdl".into(), reference: "vhdl ref".into()});
        assert_eq!(GHDL(&vhdl).definition().unwrap(), "this is vhdl");
        assert_eq!(GHDL(&vhdl).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "vhdl ref");

        vhdl.dialects.insert("ghdl".into(), CodeArch {definition: "this is ghdl".into(), reference: "ghdl ref".into()});
        assert_eq!(GHDL(&vhdl).definition().unwrap(), "this is ghdl");
        assert_eq!(GHDL(&vhdl).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "ghdl ref");
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
