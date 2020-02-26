extern crate clap;
extern crate oxs;

#[cfg(feature = "static_std")]
extern crate oxs_std;

use oxs::{
    engine::{
        Engine,
        EngineResult
    },
    codegen::{
        register::Register
    },
    api::{
        function::{
            Function
        },
        module::{
            Module
        }
    },
    parser::{
        ast::Type
    },
    vm::{
        core::Core
    }
};

use std::{
    path::Path,
    error::Error,
    boxed::Box
};

use clap::{
    App,
    SubCommand,
    Arg
};

fn bootstrap_engine(engine: &mut Engine) -> EngineResult<()> {
    oxs_std::register_extension(engine)
}


fn build_app<'a>() -> App<'a, 'a> {
    let about_string;
    #[cfg(feature = "static_std")]
    {
        about_string = "OxyScript shell script interpreter\noxs_std statically linked";
    }
    #[cfg(not(feature = "static_std"))]
    {
        about_string = "OxyScript shell script interpreter";
    }

    App::new("oxsh")
        .author("Daniel Wanner <daniel.wanner@pm.me>")
        .about(about_string)
        .version("0.1.0")
        .arg(
            Arg::with_name("filename")
                .index(1)
                .takes_value(true)
                .help("Filename of the script to execute")
        )
        .arg(
            Arg::with_name("arguments")
                .required(false)
                .takes_value(true)
                .help("Arguments to pass to the scripts main function")
                .multiple(true)
                .last(true)
        )
}

fn main() -> Result<(), Box<dyn Error>> {
    let app = build_app();

    let app_matches = app.get_matches();

    let filename_opt = app_matches.value_of("filename");
    assert!(filename_opt.is_some());

    let filename = filename_opt.unwrap();

    let mut engine = Engine::new(1024);

    let arguments_opt = app_matches.values_of("arguments");
    if arguments_opt.is_some() {
        let arguments: Vec<&str> = arguments_opt.unwrap().collect();
        for arg in arguments {
            let int_res = String::from(arg).parse::<i64>();
            let float_res = String::from(arg).parse::<f32>();

            if int_res.is_err() && float_res.is_err() {
                println!("ERROR! Not an integer or float.");
            }
            if int_res.is_ok() {
                engine.push_stack(int_res.unwrap())?;
            } else if float_res.is_ok() {
                engine.push_stack(float_res.unwrap())?;
            }
        }
    }

    #[cfg(feature = "static_std")]
    bootstrap_engine(&mut engine)?;

    engine.run_file(Path::new(filename))?;

    //println!("Script run. stack size: {}", engine.get_stack_size());

    let exit_code = engine.get_register_value::<i64>(Register::R0)?;

    //println!("Script exited. Stack size: {}, Exit code: 0x{:X}/{}", engine.get_stack_size(), exit_code, exit_code);

    std::process::exit(exit_code as i32);
}
