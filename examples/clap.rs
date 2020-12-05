fn main() {
    use clap::{load_yaml, App};

    // To load a yaml file containing our CLI definition such as the example '17_yaml.yaml' we can
    // use the convenience macro which loads the file at compile relative to the current file
    // similar to how modules are found.
    //
    // Then we pass that yaml object to App to build the CLI.
    //
    // Finally we call get_matches() to start the parsing process. We use the matches just as we
    // normally would
    let yaml = load_yaml!("cli.yml");
    let m = App::from(yaml).get_matches();

    // Because the example 17_yaml.yaml is rather large we'll just look a single arg so you can
    // see that it works...
    if let Some(mode) = m.value_of("mode") {
        match mode {
            "vi" => println!("You are using vi"),
            "emacs" => println!("You are using emacs..."),
            _ => unreachable!(),
        }
    } else {
        println!("--mode <MODE> wasn't used...");
    }
}
