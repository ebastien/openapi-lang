use anyhow::anyhow;
use clap::Parser as ClapParser;
use oal_compiler::{Locator, ModuleSet, Program};

/// Compiles a program into an OpenAPI description in YAML.
#[derive(ClapParser, Debug)]
struct Args {
    /// The path to the source program
    #[clap(short = 'i', long = "input", parse(from_os_str))]
    input: std::path::PathBuf,

    /// The path to the output OpenAPI description
    #[clap(short = 'o', long = "output", parse(from_os_str))]
    output: std::path::PathBuf,

    /// The path to a base OpenAPI description
    #[clap(short = 'b', long = "base", parse(from_os_str))]
    base: Option<std::path::PathBuf>,
}

/// Loads and parses a source file into a program.
fn loader(l: &Locator) -> anyhow::Result<Program> {
    eprintln!("Loading module {}", l);
    let path = l
        .url
        .to_file_path()
        .map_err(|_| anyhow!("not a file path: {}", l))?;
    let input = std::fs::read_to_string(path)?;
    let program = oal_syntax::parse(input)?;
    Ok(program)
}

/// Compiles a program.
fn compiler(mods: &ModuleSet, l: &Locator, p: Program) -> anyhow::Result<Program> {
    eprintln!("Compiling module {}", l);
    let program = oal_compiler::compile(mods, l, p)?;
    Ok(program)
}

fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    let main_mod = Locator::try_from(args.input.as_path())?;

    let mods = oal_compiler::load(&main_mod, loader, compiler)?;

    eprintln!("Generating API definition");

    let spec = oal_compiler::spec::Spec::try_from(&mods)?;

    let mut builder = oal_codegen::Builder::new().with_spec(spec);

    if let Some(path) = args.base {
        let file = std::fs::File::open(path)?;
        let base = serde_yaml::from_reader(file)?;
        builder = builder.with_base(base);
    }

    let api = builder.into_openapi();

    let output = serde_yaml::to_string(&api)?;

    eprintln!("Writing OpenAPI definition to {}", args.output.display());

    std::fs::write(args.output, output)?;

    Ok(())
}
