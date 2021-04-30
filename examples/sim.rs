use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use std::net::ToSocketAddrs;
use futures::AsyncReadExt;
use futures::FutureExt;
use amscircuit::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::{RefCell};
use plotters::prelude::*;

pub mod Simulator_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/api/Simulator_capnp.rs"));
}

fn plot(mut data: HashMap<String, Vec<f64>>) -> Result<(), Box<dyn std::error::Error>> {
    let time = data.remove("time").unwrap();
    let root =
        BitMapBackend::new("plot.png", (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let colors = vec![BLACK, BLUE, CYAN, GREEN, MAGENTA, RED, YELLOW];
    let colorcycle = colors.iter().cycle();

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .right_y_label_area_size(40)
        .margin(5)
        .caption("Ngspice buffer", ("sans-serif", 50.0).into_font())
        .build_cartesian_2d(0f64..2e-3f64, 0.0f64..5.064)?
        .set_secondary_coord(0f64..2e-3f64, -0.001f64..0.001f64);

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .y_desc("Voltage")
        .draw()?;

    chart
        .configure_secondary_axes()
        .y_desc("Current")
        .y_label_formatter(&|x| format!("{:e}", x))
        .draw()?;

    for ((key, val), color) in data.iter().zip(colorcycle) {
       let series = LineSeries::new(time.clone().into_iter().zip(val.clone().into_iter()), color); 
       if key.contains("#") || key.contains("@") {
            chart.draw_secondary_series(series)?
                 .label(key)
                 .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
       } else {
            chart.draw_series(series)?
                .label(key)
                .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
       }
    }

    chart
        .configure_series_labels()
        .background_style(&RGBColor(128, 128, 128))
        .draw()?;

    Ok(())
}

fn circuit() -> String {
    // PMOS transistor
    let code = CodeArch {
        reference: "m{{name}} {{port.d}} {{port.g}} {{port.s}} {{port.b}} PMOS W={{generic.w}} L={{generic.l}}".into(),
        definition: Definition::Code(".model PMOS PMOS".into())
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

    // NMOS transistor
    let code = CodeArch {
        reference: "m{{name}} {{port.d}} {{port.g}} {{port.s}} {{port.b}} NMOS W={{generic.w}} L={{generic.l}}".to_string(),
        definition: Definition::Code(".model NMOS NMOS".into())
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

    // Voltage source
    let code = CodeArch {
        reference: "v{{name}} {{port.p}} {{port.n}} {{generic.dc}} {{generic.tran}}".to_string(),
        definition: Definition::Primitive,
    };
    let mut spicemos = CodeDialectArch::new();
    spicemos.dialects.insert("spice".into(), code);
    let mut arches = HashMap::new();
    arches.insert("rtl".into(), Arch::Code(spicemos));

    let vol = Rc::from(Entity {
        name: "voltage".into(),
        symbol: Symbol {},
        generic: vec!["dc".into(), "tran".into()],
        port: vec!["p".into(), "n".into()],
        archs: arches,
    });

    // Inverter schematic
    let mut cir = Schematic {
        toplevel: false,
        instances: HashMap::new(),
    };
    cir.instances.insert(
            "pmos".into(),
            Instance {
                genericmap: collection!{
                    "w".into() => "1u".into(),
                    "l".into() => "1u".into(),
                },
                portmap: collection!{
                    "g".into() => "in".into(),
                    "d".into() => "out".into(),
                    "s".into() => "vdd".into(),
                    "b".into() => "vdd".into(),
                },
                x: 0,
                y: 0,
                entity: pmos.clone(),
            });
    cir.instances.insert(
            "nmos".into(),
            Instance {
                genericmap: collection!{
                    "w".into() => "1u".into(),
                    "l".into() => "1u".into(),
                },
                portmap: collection!{
                    "g".into() => "in".into(),
                    "d".into() => "out".into(),
                    "s".into() => "gnd".into(),
                    "b".into() => "gnd".into(),
                },
                x: 0,
                y: 0,
                entity: nmos.clone(),
            });
    // Inverter entity
    let inv = Rc::from(Entity {
        name: "inverter".into(),
        symbol: Symbol {},
        port: vec!["vdd".into(), "gnd".into(), "in".into(), "out".into()],
        generic: Vec::new(),
        archs: collection!{"default".into() => Arch::Schematic(cir)},
    });

    // Buffer schematic
    let mut cir = Schematic {
        toplevel: false,
        instances: HashMap::new(),
    };
    cir.instances.insert(
            "inv1".into(),
            Instance {
                genericmap: HashMap::new(),
                portmap: collection!{
                    "in".into() => "in".into(),
                    "out".into() => "mid".into(),
                    "vdd".into() => "vdd".into(),
                    "gnd".into() => "gnd".into(),
                },
                x: 0,
                y: 0,
                entity: inv.clone(),
            });
    cir.instances.insert(
            "inv2".into(),
            Instance {
                genericmap: HashMap::new(),
                portmap: collection!{
                    "in".into() => "mid".into(),
                    "out".into() => "out".into(),
                    "vdd".into() => "vdd".into(),
                    "gnd".into() => "gnd".into(),
                },
                x: 0,
                y: 0,
                entity: inv.clone(),
            });

    // Buffer entity
    let buf = Rc::from(Entity {
        name: "buffer".into(),
        symbol: Symbol {},
        port: vec!["vdd".into(), "gnd".into(), "in".into(), "out".into()],
        generic: Vec::new(),
        archs: collection!{"default".into() => Arch::Schematic(cir)},
    });

    // Testbench schematic
    let mut cir = Schematic {
        toplevel: true,
        instances: HashMap::new(),
    };
    cir.instances.insert(
            "buf".into(),
            Instance {
                genericmap: HashMap::new(),
                portmap: collection!{
                    "in".into() => "in".into(),
                    "out".into() => "out".into(),
                    "vdd".into() => "vdd".into(),
                    "gnd".into() => "gnd".into(),
                },
                x: 0,
                y: 0,
                entity: buf.clone(),
            });
    cir.instances.insert(
            "input".into(),
            Instance {
                genericmap: collection!{
                    "dc".into() => "0".into(),
                    "tran".into() => "sin(2.5 2.5 1k)".into(),
                },
                portmap: collection!{
                    "p".into() => "in".into(),
                    "n".into() => "gnd".into(),
                },
                x: 0,
                y: 0,
                entity: vol.clone(),
            });

    cir.instances.insert(
            "supply".into(),
            Instance {
                genericmap: collection!{
                    "dc".into() => "5".into(),
                },
                portmap: collection!{
                    "p".into() => "vdd".into(),
                    "n".into() => "gnd".into(),
                },
                x: 0,
                y: 0,
                entity: vol.clone(),
            });

    // Testbench entity
    let tb = Rc::from(Entity {
        name: "tb".into(),
        symbol: Symbol {},
        port: Vec::new(),
        generic: Vec::new(),
        archs: collection!{"default".into() => Arch::Schematic(cir)},
    });

    let conf = Configuration {
        sim: Ngspice,
        ent: tb,
        arch: Some("default".into()),
        for_inst: RefCell::from(HashMap::new()),
        all: HashMap::new(),
    };
    if let Definition::Code(code) = &conf.definition().unwrap()[0] {
        println!("{}", code);
        return code.into();
    } else {
        return "".into()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cir = circuit();
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 2 {
        println!("usage: {} HOST:PORT", args[0]);
        return Ok(());
    }

    let addr = args[1]
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");

    tokio::task::LocalSet::new().run_until(async move {
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        stream.set_nodelay(true)?;
        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
        let rpc_network = Box::new(twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        ));
        let mut rpc_system = RpcSystem::new(rpc_network, None);
        let sim: Simulator_capnp::simulator::Client<Simulator_capnp::tran::Owned> =
            rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));

        let mut request = sim.load_files_request();
        let mut file = request.get().init_files(1).get(0);
        file.set_name("rc.sp");
        file.set_contents(cir.as_bytes());

        let reply = request.send().promise.await.unwrap();

        let cmd = reply.get().unwrap().get_commands().unwrap();
        let mut request = cmd.tran_request();
        let mut param = request.get();
        param.set_step(1e-6);
        param.set_start(0.0);
        param.set_stop(2e-3);

        let reply = request.send().promise.await.unwrap();

        let res = reply.get().unwrap().get_result().unwrap();

        let mut resdict: HashMap<String, Vec<f64>> = HashMap::new();

        loop {
            let reply = res.read_request().send().promise.await.unwrap();
            let reply_data = reply.get().unwrap();
            let data = reply_data.get_data().unwrap();
            let more = reply_data.get_more();
            for vec in data {
                let name = vec.get_name().unwrap();
                let data = vec.get_data();
                // println!("{}", name);
                match data.which().unwrap() {
                    Simulator_capnp::vector::data::Real(data) => for item in data.unwrap() {
                        resdict.entry(name.into()).or_insert(Vec::new()).push(item);
                    },
                    _ => println!("other data")
                }
            }
            if !more {
                break;
            }
        }

        plot(resdict)?;

        Ok(())
    }).await
}
