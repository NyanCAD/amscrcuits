extern crate capnpc;

fn main() {
    ::capnpc::CompilerCommand::new().file("src/api/Simulator.capnp").run().unwrap();
}
