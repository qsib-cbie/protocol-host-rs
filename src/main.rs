extern crate clap;
extern crate log;
extern crate simple_logger;

use clap::{Arg, App};

fn main() -> Result<(), std::io::Error> {
    // Define the acceptable user input behavior
    let matches = App::new("VR Actuators")
        .version("v0.1")
        .author("Jacob Trueb <jtrueb@northwestern.edu")
        .about("Manipulate VR Actuator Blocks")
        .arg(Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .subcommand(App::new("start")
            .about("Starts the service that manages the connection to the VR Actuators")
            .arg(Arg::with_name("hostname")
                .short("h")
                .long("hostname")
                .value_name("HOSTNAME")
                .default_value("*")
                .help("Sets hostname to bind for communication")
                .takes_value(true))
            .arg(Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("5555")
                .help("Sets port to bind for communication")
                .takes_value(true))
            .arg(Arg::with_name("protocol")
                .long("protocol")
                .value_name("PROTOCOL")
                .default_value("tcp")
                .help("Sets ZMQ protocol for the server")
                .takes_value(true))
            .arg(Arg::with_name("threads")
                .long("threads")
                .value_name("THREADS")
                .default_value("1")
                .help("Sets the number of ZMQ servers starting from PORT")
                .takes_value(true)))
        .subcommand(App::new("command")
            .about("Executes a command for VR Actuators")
            .arg(Arg::with_name("hostname")
                .short("h")
                .long("hostname")
                .value_name("HOSTNAME")
                .default_value("localhost")
                .help("Sets hostname to bind for communication")
                .takes_value(true))
            .arg(Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("5555")
                .help("Sets port to bind for communication")
                .takes_value(true))
            .arg(Arg::with_name("protocol")
                .long("protocol")
                .value_name("PROTOCOL")
                .default_value("tcp")
                .help("Sets ZMQ protocol for the server")
                .takes_value(true))
            .arg(Arg::with_name("id")
                .long("id")
                .value_name("ID")
                .default_value("0")
                .help("Sets an id for the command")
                .takes_value(true)))
        .get_matches();

    // Configure the logger before heading off to the rest of the functionality
    simple_logger::init().unwrap(); 
    let level_filter = match matches.occurrences_of("v") {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        3 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Trace,
    };
    log::set_max_level(level_filter);
    log::debug!("Found level_filter: {}", level_filter);

    // Kick off logic for the subcommands and configuration
    if let Some(matches) = matches.subcommand_matches("start") {
        log::info!("Starting up ...");
        log::trace!("Start Params: {:#?}", matches);

        // Start listening for connections
        let protocol = String::from(matches.value_of("protocol").unwrap());
        let hostname = String::from(matches.value_of("hostname").unwrap());
        let port = String::from(matches.value_of("port").unwrap());
        let threads = String::from(matches.value_of("threads").unwrap());

        let port: i32 = port.parse().expect("Expected integer for port");
        let threads: i32 = threads.parse().expect("Expected integer for threads");

        let ctx = zmq::Context::new();
        let mut join_handles: std::vec::Vec<_> = std::vec::Vec::new();
        log::info!("Spawning {} servers ...", threads);
        for i in 0..threads {
            log::info!("Spawning server thread {} ...", i);
            let ctx = ctx.clone();
            let protocol = protocol.clone();
            let hostname = hostname.clone();
            let port = port;

            join_handles.push(std::thread::spawn(move || -> Result<(), std::io::Error> {
                log::trace!("Beginning thread {} ...", i);
                let socket = ctx.socket(zmq::REP)?;
                log::trace!("Created socket");

                let addr = format!("{}://{}:{}", protocol, hostname, (port + i).to_string());
                socket.bind(addr.as_str())?;
                log::info!("Bound socket on {}", addr);

                let mut message = zmq::Message::new();
                loop {
                    socket.recv(&mut message, 0)?;
                    let message = message.as_str().unwrap();
                    log::debug!("Received: {}", message);

                    socket.send(message.as_bytes(), 0)?;
                    log::debug!("Sent: {}", message);
                }
            }));
        }

        // Wait forever
        for handle in join_handles {
            handle.join().expect("Failed to join thread that never should have joined")?;
        }
        
    } else if let Some(matches) = matches.subcommand_matches("command") {
        log::info!("Running command: {}", "command");
        log::trace!("Command Params: {:#?}", matches);

        // Start listening for connections
        let protocol = String::from(matches.value_of("protocol").unwrap());
        let hostname = String::from(matches.value_of("hostname").unwrap());
        let port = String::from(matches.value_of("port").unwrap());
        let id = String::from(matches.value_of("id").unwrap());

        let port: i32 = port.parse().expect("Expected integer for port");
        let id: i32 = id.parse().expect("Expected integer for id");

        let ctx = zmq::Context::new();
        let socket = ctx.socket(zmq::REQ)?;
        log::trace!("Created socket");
        let addr = format!("{}://{}:{}", protocol, hostname, (port + id % 8).to_string());
        socket.connect(addr.as_str())?;
        log::info!("Connected socket to {}", addr);

        for i in 0..=100000 {
            let message = format!("Hello, World {}", i);
            socket.send(message.as_bytes(), 0)?;
            log::debug!("Sent: {}", message);

            let mut message = zmq::Message::new();
            socket.recv(&mut message, 0)?;
            log::debug!("Received: {}", message.as_str().unwrap());
        }
    } else {
        log::error!("Unknown command. Exiting ...");
        std::process::exit(1);
    }

    Ok(())
}