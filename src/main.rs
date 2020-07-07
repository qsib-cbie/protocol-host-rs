extern crate clap;
extern crate log;
extern crate simple_logger;

use clap::{Arg, App};

fn main() {
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
                .help("Sets hostname to bind for communication")
                .takes_value(true))
            .arg(Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .help("Sets port to bind for communication")
                .takes_value(true)))
        .subcommand(App::new("stop")
            .about("Shuts down any connection to VR Actuators"))
        .get_matches();

    // Configure the logger before heading off to the rest of the functionality
    simple_logger::init().unwrap(); 
    let level_filter = match matches.occurrences_of("v") {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        3 | _ => log::LevelFilter::Trace,
    };
    log::set_max_level(level_filter);
    log::debug!("Found level_filter: {}", level_filter);

    // Kick off logic for the subcommands and configuration
    if let Some(matches) = matches.subcommand_matches("start") {
        log::info!("Starting up ...");
        log::trace!("Start Params: {:#?}", matches)
    } else if let Some(matches) = matches.subcommand_matches("stop") {
        log::info!("Shutting down ...");
        log::trace!("Stop Params: {:#?}", matches)
    } else {
        log::error!("Unknown command. Exiting ...");
        std::process::exit(1)
    }
}