
mod vrp;
mod network;
mod conn;
mod obid;
mod error;

use error::{Result, InternalError};
use conn::common::*;

fn start_server<'a>(conn_type: &str, protocol: &str, hostname: &str, port: i16) -> Result<()> {
    let endpoint = network::common::NetworkContext::get_endpoint(protocol, hostname, port);

    // Create various contexts needed for hardware interaction
    let libusb_context = libusb::Context::new()?;
    let server_context = network::server::ServerContext::new(endpoint)?;

    match conn_type {
        "mock" => {
            let context = Box::new(conn::mock::MockContext::new());
            let connection = context.connection()?;
            start_server_with_connection(&connection, &server_context)
        },
        "usb" => {
            let context = Box::new(conn::usb::UsbContext::new(&libusb_context)?);
            let connection = context.connection()?;
            start_server_with_connection(&connection, &server_context)
        },
        err => {
            log::error!("Invalid connection type: {} not supported", err);
            return Err(InternalError::from(String::from("Invalid connection Type")));
        }
    }
}

// fn start_server_with_context<'a, 'b>(context: &'a impl conn::common::Context<'a>, server_context: &'b network::server::ServerContext) -> Result<()> {
//     let connection  = context.connection()?;
//     start_server_with_connection(&connection, server_context)
// }

fn start_server_with_connection<'a, 'b>(connection: &'a impl conn::common::Connection<'a>, server_context: &'b network::server::ServerContext) -> Result<()> {
    let mut server = network::server::Server::new(server_context, connection).expect("Failed to initialize server");
    match server.serve() {
        Ok(reserve) => {
            log::info!("Finished serving with Ok result.");
            if !reserve {
                return Ok(());
            }
        },
        Err(err) => {
            log::error!("Encountered error: {}", err);
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    // Define the acceptable user input behavior
    let matches = clap::App::new("VR Actuators")
        .version("v0.1")
        .author("Jacob Trueb <jtrueb@northwestern.edu")
        .about("Manipulate VR Actuator Blocks")
        .arg(clap::Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .subcommand(clap::App::new("start")
            .about("Starts the service that manages the connection to the VR Actuators")
            .arg(clap::Arg::with_name("conn_type")
                .short("c")
                .long("conn-type")
                .value_name("CONN_TYPE")
                .default_value("usb")
                .help("The type of connection that will be attempted")
                .takes_value(true))
            .arg(clap::Arg::with_name("hostname")
                .short("h")
                .long("hostname")
                .value_name("HOSTNAME")
                .default_value("*")
                .help("Sets hostname to bind for communication")
                .takes_value(true))
            .arg(clap::Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("5555")
                .help("Sets port to bind for communication")
                .takes_value(true))
            .arg(clap::Arg::with_name("protocol")
                .long("protocol")
                .value_name("PROTOCOL")
                .default_value("tcp")
                .help("Sets ZMQ protocol for the server")
                .takes_value(true)))
        .subcommand(clap::App::new("command")
            .about("Executes a command for VR Actuators")
            .arg(clap::Arg::with_name("hostname")
                .short("h")
                .long("hostname")
                .value_name("HOSTNAME")
                .default_value("localhost")
                .help("Sets hostname to bind for communication")
                .takes_value(true))
            .arg(clap::Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("5555")
                .help("Sets port to bind for communication")
                .takes_value(true))
            .arg(clap::Arg::with_name("protocol")
                .long("protocol")
                .value_name("PROTOCOL")
                .default_value("tcp")
                .help("Sets ZMQ protocol for the server")
                .takes_value(true))
            .arg(clap::Arg::with_name("commands")
                .required(true)
                .value_name("COMMANDS_FILE")
                .default_value("commands.txt")
                .index(1)
                .help("Sets the file that lists the commands to execute")))
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
        let conn_type = matches.value_of("conn_type").unwrap();
        let protocol = matches.value_of("protocol").unwrap();
        let hostname = matches.value_of("hostname").unwrap();
        let port = matches.value_of("port").unwrap();
        let port: i16 = port.parse().expect("Expected a small integer for port");

        loop {
            start_server(conn_type, protocol, hostname, port)?;
        }
    } else if let Some(matches) = matches.subcommand_matches("command") {
        log::info!("Running command: {}", "command");
        log::trace!("Command Params: {:#?}", matches);

        // Start listening for connections
        let protocol = String::from(matches.value_of("protocol").unwrap());
        let hostname = String::from(matches.value_of("hostname").unwrap());
        let port = String::from(matches.value_of("port").unwrap());
        let port: i16 = port.parse().expect("Expected small integer for port");

        let endpoint = network::common::NetworkContext::get_endpoint(protocol.as_str(), hostname.as_str(), port);
        let mut client = network::client::Client::new(endpoint).expect("Failed to initialize client");

        // Send each of the commands
        let commands = String::from(matches.value_of("commands").unwrap());
        let file = std::fs::File::open(commands)?;
        let reader = std::io::BufReader::new(file);
        let stream = serde_json::Deserializer::from_reader(reader).into_iter::<network::common::CommandMessage>();
        for command in stream {
            log::trace!("Found command: {:#?}", command);
            client.request_message(command?)?
        }
    } else {
        log::error!("Unknown command. Exiting ...");
        std::process::exit(1);
    }

    Ok(())
}