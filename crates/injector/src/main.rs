use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use sandblaster_disasm::IcedX86Disassembler;
use sandblaster_injector::{
    BackendObservation, ExecutionBackend, InjectorConfig, InjectorEngine, InjectorEvent,
    LinuxX86Backend, OutputMode, RawInjectorPacket, TextReport,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-?" || arg == "--help") {
        print!("{}", InjectorConfig::help_text());
        return ExitCode::SUCCESS;
    }

    match InjectorConfig::parse_args(&args) {
        Ok(config) => {
            if config.dry_run {
                let mut engine = InjectorEngine::new(IcedX86Disassembler, DryRunBackend, &config);
                run_engine(&mut engine, &config)
            } else {
                let backend = match LinuxX86Backend::from_config(&config) {
                    Ok(backend) => backend,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                };
                let mut engine = InjectorEngine::new(IcedX86Disassembler, backend, &config);
                run_engine(&mut engine, &config)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

struct DryRunBackend;

impl ExecutionBackend for DryRunBackend {
    fn execute(
        &mut self,
        instruction: &sandblaster_core::InstructionBytes,
    ) -> Result<BackendObservation, String> {
        Ok(BackendObservation {
            valid: 1,
            length: instruction.specified_len() as u32,
            signum: 5,
            si_code: 0,
            fault_addr: u32::MAX,
        })
    }
}

fn run_engine<D, E>(engine: &mut InjectorEngine<D, E>, config: &InjectorConfig) -> ExitCode
where
    D: sandblaster_disasm::DisasmBackend,
    E: sandblaster_injector::ExecutionBackend,
{
    loop {
        match engine.next_event() {
            Ok(Some(InjectorEvent::Executed(result))) => {
                if emit_result(&result, config.output_mode).is_err() {
                    return ExitCode::from(1);
                }
            }
            Ok(Some(InjectorEvent::Skipped(result, reason))) => {
                if emit_result(&result, config.output_mode).is_err() {
                    return ExitCode::from(1);
                }
                if matches!(config.output_mode, OutputMode::Text) {
                    eprintln!("skipped candidate: {reason}");
                }
            }
            Ok(None) => return ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
        }
    }
}

fn emit_result(
    result: &sandblaster_core::ExecutionResult,
    output_mode: OutputMode,
) -> io::Result<()> {
    match output_mode {
        OutputMode::Raw => {
            let packet = RawInjectorPacket::from_execution_result(result);
            io::stdout().write_all(&packet.to_bytes())
        }
        OutputMode::Text => {
            let report = TextReport::from_execution_result(result);
            print!("{}", report.0);
            Ok(())
        }
    }
}
