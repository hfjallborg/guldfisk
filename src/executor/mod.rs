use crossbeam_channel::Receiver;

pub fn run(receiver: Receiver<String>) {
    // Receives commands from connection threads and executes them.

    for msg in receiver.iter() {
        println!("{}", msg);
    }
}
