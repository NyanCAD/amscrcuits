use std::collections::HashMap;
use std::rc::Rc;
use std::cell::{Ref, RefCell};
use std::path::PathBuf;
use handlebars::Handlebars;
use serde::Serialize;
use indexmap::{indexset, IndexSet};

/// Macro for HashMap literals
#[macro_export]
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

pub struct Configuration<S: Simulator> {
    /// The simulator to target
    pub sim: S,
    /// The entity to synthesize
    pub ent: Rc<Entity>,
    /// The architecture to use for this entity.
    /// If None, a default from all is used, or the first that matches the simulator
    pub arch: Option<String>,
    // The configuration for a sub-instance
    pub for_inst: RefCell<HashMap<String, Configuration<S>>>,
    /// For all Entity => Arch.
    /// Weakest specification.
    pub all: HashMap<String, String>,
}

impl<S> Configuration<S> where S: Simulator {
    fn get_arch(&self) -> Option<&Arch> {
        if let Some(arch) = &self.arch { // directly specified
            self.ent.archs.get(arch)
        } else if let Some(arch) = self.all.get(&self.ent.name) { // entity specified
            self.ent.archs.get(arch)
        } else { // find the first one that supports this sim
            for (_name, arch) in &self.ent.archs {
                match arch {
                    Arch::Code(cda) => if self.sim.get_dialect(cda).is_some() {
                        return Some(arch);
                    }
                    Arch::Schematic(_) => return Some(arch)
                }
            }
            None
        }
    }

    /// Gets the configuration for a certain instance.
    /// If no configuration is given for this instance,
    /// a default configuration is created with a copy of
    /// the per-entity defaults
    fn get_conf(&self, name: &str, inst: &Instance) -> Ref<Configuration<S>> {
        self.for_inst.borrow_mut().entry(name.into()).or_insert_with(|| Configuration {
            sim: self.sim,
            ent: inst.entity.clone(),
            arch: None,
            for_inst: RefCell::from(HashMap::new()),
            all: self.all.clone(),
        });
        Ref::map(self.for_inst.borrow(), |inst| &inst[name.into()])
    }
}

// TODO instances and schematics require a complete rework for GUI interface
pub struct Instance {
    pub portmap: HashMap<String, String>,
    pub genericmap: HashMap<String, String>,
    pub x: i64,
    pub y: i64,
    pub entity: Rc<Entity>,
}

pub struct Schematic {
    pub toplevel: bool,
    pub instances: HashMap<String, Instance>,
}

/// Represents a component that can be expressed in code.
/// For example, a spice model/subcircuit or a VHDL architecture.
pub trait Code {
    /// The definition of this component.
    /// The .subckt or architecture code
    fn definition(&self) -> Result<IndexSet<Definition>, CodeError> { Err(CodeError::DialectError) }
    /// The declaration, not used in all languages, and by default will return a DialectError.
    /// In VHDL this is the component declaration in the instantiating architecture.
    /// Component instantiation is required for configurations
    fn declaration(&self) -> Result<String, CodeError> { Err(CodeError::DialectError) }
    /// The reference to a component given the instance name, and the ports and parameters to pass to the component.
    /// This is used to instantiate a component in another one.
    fn reference(&self, _name: &str, _genericmap: &HashMap<String, String>, _portmap: &HashMap<String, String>) -> Result<String, CodeError> { Err(CodeError::DialectError) }
}

#[derive(Debug)]
pub enum CodeError {
    DialectError,
    CompileError(String),
    TemplateError(handlebars::TemplateRenderError),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Definition {
    Code(String),
    Library(PathBuf),
    Primitive,
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

/// Contains a definition in some language
/// and a Handlebars template for referencing the definition
pub struct CodeArch {
    pub definition: Definition,
    pub reference: String,
}

impl Code for CodeArch {
    fn definition(&self) -> Result<IndexSet<Definition>, CodeError> { Ok(indexset!{self.definition.clone()}) }
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
    pub dialects: HashMap<String, CodeArch>,
}

impl CodeDialectArch {
    pub fn new() -> CodeDialectArch {
        CodeDialectArch {dialects: HashMap::new()}
    }
}

pub trait Simulator: Copy {
    fn get_dialect<'a>(&self, arch: &'a CodeDialectArch) -> Option<&'a CodeArch>;
    fn synthesize_definition<S: Simulator>(&self, conf: &Configuration<S>, ckt: &Schematic) -> Result<IndexSet<Definition>, CodeError>;
    fn synthesize_reference<S: Simulator>(&self, conf: &Configuration<S>, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError>;
}

fn spice_definition<S: Simulator>(sch: &Schematic, conf: &Configuration<S>) -> Result<IndexSet<Definition>, CodeError> {
    let mut defs = IndexSet::new();
    if sch.toplevel {
        let mut res = String::new();
        res.push_str(&format!("* {}\n", conf.ent.name));
        let mut sub_defs = IndexSet::new();
        for (name, inst) in &sch.instances {
            let subconf = conf.get_conf(name, inst);
            // add to ordered set to avoid duplicates but maintain dependency order
            sub_defs.extend(subconf.definition()?)
        }
        for def in sub_defs {
            match def {
                Definition::Code(def) => res.push_str(&def),
                Definition::Library(lib) => res.push_str(&format!(".lib {}", lib.to_str().ok_or(CodeError::CompileError(lib.to_string_lossy().into()))?)),
                Definition::Primitive => (),
            }
            res.push('\n');
        }
        for (name, inst) in &sch.instances {
            let subconf = conf.get_conf(name, inst);
            res.push_str(&subconf.reference(name, &inst.genericmap, &inst.portmap)?);
            res.push('\n');
        }
        res.push_str(".end\n");
        defs.insert(Definition::Code(res));
    } else {
        for (name, inst) in &sch.instances {
            let subconf = conf.get_conf(name, inst);
            // add to ordered set to avoid duplicates but maintain dependency order
            defs.extend(subconf.definition()?)
        }
        let mut res = String::new();
        res.push_str(&format!(".subckt {}", conf.ent.name));
        for port in &conf.ent.port {
            res.push(' ');
            res.push_str(port);
        }
        res.push('\n');
        // TODO parameters
        for (name, inst) in &sch.instances {
            let subconf = conf.get_conf(name, inst);
            res.push_str(&subconf.reference(name, &inst.genericmap, &inst.portmap)?);
            res.push('\n');
        }
        res.push_str(&format!(".ends {}", conf.ent.name));
        defs.insert(Definition::Code(res));
    }
    Ok(defs)
}
fn spice_reference<S: Simulator>(conf: &Configuration<S>, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError> {
    let mut res = String::with_capacity(64);
    res.push('x');
    res.push_str(name);
    // order matters
    for p in &conf.ent.port {
        res.push(' ');
        res.push_str(portmap.get(p).ok_or(CodeError::CompileError(format!("no {} in {}", p, name)))?)
    }
    res.push(' ');
    res.push_str(&conf.ent.name);
    for g in &conf.ent.generic {
        res.push(' ');
        res.push_str(g);
        res.push('=');
        res.push_str(genericmap.get(g).ok_or(CodeError::CompileError(g.into()))?)
    }
    Ok(res)
}


#[derive(Copy, Clone)]
pub struct Ngspice;

impl Simulator for Ngspice {
    fn get_dialect<'a>(&self, arch: &'a CodeDialectArch) -> Option<&'a CodeArch> {
        arch.dialects.get("ngspice").or_else(|| arch.dialects.get("spice"))
    }
    fn synthesize_definition<S: Simulator>(&self, conf: &Configuration<S>, ckt: &Schematic) -> Result<IndexSet<Definition>, CodeError> {
        spice_definition(ckt, conf)
    }
    fn synthesize_reference<S: Simulator>(&self, conf: &Configuration<S>, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError> {
        spice_reference(conf, name, genericmap, portmap)
    }
}

// pub struct Xyce;
// pub struct Verilator;
// pub struct GHDL;

// CXXRTL takes anything Yosys can read plus C++
// pub struct CXXRTL;

// nMigen takes Python files
// pub struct NMigen;

impl<S: Simulator> Code for Configuration<S> {
    fn definition(&self) -> Result<IndexSet<Definition>, CodeError> {
        match self.get_arch() {
            Some(Arch::Code(arch)) => self.sim.get_dialect(arch).ok_or(CodeError::DialectError)?.definition(),
            Some(Arch::Schematic(sch)) => self.sim.synthesize_definition(self, sch),
            None => Err(CodeError::DialectError)
        }
    }
    fn reference(&self, name: &str, genericmap: &HashMap<String, String>, portmap: &HashMap<String, String>) -> Result<String, CodeError> {
        match self.get_arch() {
            Some(Arch::Code(arch)) => self.sim.get_dialect(arch).ok_or(CodeError::DialectError)?.reference(name, genericmap, portmap),
            Some(Arch::Schematic(_sch)) => self.sim.synthesize_reference(self, name, genericmap, portmap),
            None => Err(CodeError::DialectError)
        }
    }
}

#[cfg(test)]
mod tests {
    use vhdl_lang::{VHDLParser, Diagnostic, ast};
    use std::path::Path;
    use super::*;

    #[test]
    fn circuit() {
        let code = CodeArch {
            reference: "m{{name}} {{port.d}} {{port.g}} {{port.s}} {{port.b}} PMOS W={{generic.w}} L={{generic.l}}".into(),
            definition: Definition::Code(".model PMOS".into())
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
            definition: Definition::Code(".model NMOS".into())
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
            toplevel: true,
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
        let top = Entity {
            name: "buf".into(),
            symbol: Symbol {},
            port: vec!["vdd".into(), "gnd".into(), "in".into(), "out".into()],
            generic: Vec::new(),
            archs: collection!{"default".into() => Arch::Schematic(cir)},
        };
        let conf = Configuration {
            sim: Ngspice,
            ent: Rc::from(top),
            arch: Some("default".into()),
            for_inst: RefCell::from(HashMap::new()),
            all: HashMap::new(),
        };
        if let Definition::Code(code) = &conf.definition().unwrap()[0] {
            println!("{}", code);
        }
        // assert_eq!(Ngspice(&cir).definition().unwrap(), "");
    }

    #[test]
    fn code_arch() {
        let code = CodeArch {
            reference: "{{generic.name}}, {{port.platitude}}".to_string(),
            definition: Definition::Code("hello".into())};
        let mut generics = HashMap::new();
        generics.insert("name".to_string(), "world".to_string());
        let mut ports = HashMap::new();
        ports.insert("platitude".to_string(), "whatsup".to_string());
        assert_eq!(code.definition().unwrap(), indexset!{Definition::Code("hello".into())});
        assert_eq!(code.reference("foo", &generics, &ports).unwrap(), "world, whatsup");
    }

    // #[test]
    // fn spice_arch() {
    //     let mut spice = CodeDialectArch::new();
    //     spice.dialects.insert("spice".into(), CodeArch {definition: "this is spice".into(), reference: "spice ref".into()});
    //     spice.dialects.insert("ngspice".into(), CodeArch {definition: "this is ngspice".into(), reference: "ngspice ref".into()});
    //     assert_eq!(Ngspice(&spice).definition().unwrap(), "this is ngspice");
    //     assert_eq!(Xyce(&spice).definition().unwrap(), "this is spice");
    //     assert_eq!(Ngspice(&spice).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "ngspice ref");
    //     assert_eq!(Xyce(&spice).reference("bar", &HashMap::new(), &HashMap::new()).unwrap(), "spice ref");


    //     let mut spice = CodeDialectArch::new();
    //     spice.dialects.insert("spice".into(), CodeArch {definition: "this is spice".into(), reference: "spice ref".into()});
    //     spice.dialects.insert("xyce".into(), CodeArch {definition: "this is xyce".into(), reference: "xyce ref".into()});
    //     assert_eq!(Ngspice(&spice).definition().unwrap(), "this is spice");
    //     assert_eq!(Xyce(&spice).definition().unwrap(), "this is xyce");
    //     assert_eq!(Ngspice(&spice).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "spice ref");
    //     assert_eq!(Xyce(&spice).reference("bar", &HashMap::new(), &HashMap::new()).unwrap(), "xyce ref");
    // }

    // #[test]
    // fn verilator_arch() {
    //     let mut verilog = CodeDialectArch::new();
    //     verilog.dialects.insert("verilog".into(), CodeArch {definition: "this is verilog".into(), reference: "verilog ref".into()});
    //     assert_eq!(Verilator(&verilog).definition().unwrap(), "this is verilog");
    //     assert_eq!(Verilator(&verilog).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "verilog ref");

    //     verilog.dialects.insert("verilator".into(), CodeArch {definition: "this is verilator".into(), reference: "verilator ref".into()});
    //     assert_eq!(Verilator(&verilog).definition().unwrap(), "this is verilator");
    //     assert_eq!(Verilator(&verilog).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "verilator ref");
    //     assert_eq!(Ngspice(&verilog).definition().is_err(), true);
    // }

    // #[test]
    // fn ghdl_arch() {
    //     let mut vhdl = CodeDialectArch::new();
    //     vhdl.dialects.insert("vhdl".into(), CodeArch {definition: "this is vhdl".into(), reference: "vhdl ref".into()});
    //     assert_eq!(GHDL(&vhdl).definition().unwrap(), "this is vhdl");
    //     assert_eq!(GHDL(&vhdl).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "vhdl ref");

    //     vhdl.dialects.insert("ghdl".into(), CodeArch {definition: "this is ghdl".into(), reference: "ghdl ref".into()});
    //     assert_eq!(GHDL(&vhdl).definition().unwrap(), "this is ghdl");
    //     assert_eq!(GHDL(&vhdl).reference("foo", &HashMap::new(), &HashMap::new()).unwrap(), "ghdl ref");
    //     assert_eq!(Xyce(&vhdl).definition().is_err(), true);
    // }

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
