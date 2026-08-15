use std::collections::VecDeque;
use std::convert::Infallible;

use crate::{BusRegion, CompilerVm, CpuError, CpuRegisters, CpuStep};

/// Lifecycle hooks around each CPU instruction executed by [`VmRunner`].
///
/// Pre-step hooks may mutate the VM to deliver scheduled input or perform other
/// host-side actions. Post-step hooks observe only successfully completed
/// instructions and run before history retention and stop-condition evaluation.
pub trait VmRunHooks {
    type Error;

    fn before_step(&mut self, _vm: &mut CompilerVm) -> Result<(), Self::Error> {
        Ok(())
    }

    fn after_step(&mut self, _vm: &CompilerVm, _step: &CpuStep) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Default)]
struct NoopHooks;

impl VmRunHooks for NoopHooks {
    type Error = Infallible;
}

/// Execution policy for the library-owned VM step loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunRequest {
    /// Maximum number of calls to the CPU step function.
    pub max_steps: u64,
    /// Stop after executing an instruction whose starting PC matches this value.
    pub stop_after_pc: Option<u16>,
    /// Number of successfully completed instructions retained in the report.
    pub history_len: usize,
}

impl Default for RunRequest {
    fn default() -> Self {
        Self {
            max_steps: 1_000,
            stop_after_pc: None,
            history_len: 64,
        }
    }
}

/// Structured reason why a VM run stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    StepLimit {
        max_steps: u64,
    },
    PcReached {
        pc: u16,
    },
    Halted,
    UnsupportedOpcode {
        pc: u16,
        opcode: u8,
    },
    ProtectedCodeWrite {
        pc: u16,
        address: u16,
        old_value: u8,
        new_value: u8,
        region: BusRegion,
    },
}

impl From<CpuError> for StopReason {
    fn from(error: CpuError) -> Self {
        match error {
            CpuError::Halted => Self::Halted,
            CpuError::UnsupportedOpcode { pc, opcode } => Self::UnsupportedOpcode { pc, opcode },
            CpuError::ProtectedCodeWrite {
                pc,
                address,
                old_value,
                new_value,
                region,
            } => Self::ProtectedCodeWrite {
                pc,
                address,
                old_value,
                new_value,
                region,
            },
        }
    }
}

/// Execution facts that do not require parsing CLI output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    pub stop: StopReason,
    /// Number of calls made to the CPU step function, including a failed call.
    pub attempted_steps: u64,
    /// Number of calls that returned a completed instruction.
    pub completed_steps: u64,
    pub cycles: u64,
    pub registers: CpuRegisters,
    pub history: Vec<CpuStep>,
}

/// A completed run together with its directly inspectable final VM state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    pub report: RunReport,
    pub vm: CompilerVm,
}

impl RunOutcome {
    pub fn stop_reason(&self) -> StopReason {
        self.report.stop
    }

    pub fn memory(&self) -> &crate::Memory {
        self.vm.memory()
    }

    pub fn into_vm(self) -> CompilerVm {
        self.vm
    }
}

/// Owns a prepared VM and executes it under a structured run policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmRunner {
    vm: CompilerVm,
}

impl VmRunner {
    pub fn new(vm: CompilerVm) -> Self {
        Self { vm }
    }

    pub fn vm(&self) -> &CompilerVm {
        &self.vm
    }

    pub fn vm_mut(&mut self) -> &mut CompilerVm {
        &mut self.vm
    }

    pub fn into_vm(self) -> CompilerVm {
        self.vm
    }

    pub fn run(self, request: RunRequest) -> RunOutcome {
        let result = self.run_with_hooks(request, &mut NoopHooks);
        match result {
            Ok(outcome) => outcome,
            Err(error) => match error {},
        }
    }

    pub fn run_with_hooks<H>(
        mut self,
        request: RunRequest,
        hooks: &mut H,
    ) -> Result<RunOutcome, H::Error>
    where
        H: VmRunHooks,
    {
        let mut history = VecDeque::with_capacity(request.history_len);
        let mut attempted_steps = 0;
        let mut completed_steps = 0;

        let stop = loop {
            if attempted_steps == request.max_steps {
                break StopReason::StepLimit {
                    max_steps: request.max_steps,
                };
            }

            hooks.before_step(&mut self.vm)?;
            attempted_steps += 1;
            match self.vm.step_cpu() {
                Ok(step) => {
                    completed_steps += 1;
                    hooks.after_step(&self.vm, &step)?;
                    push_history(&mut history, request.history_len, step);
                    if request.stop_after_pc == Some(step.pc) {
                        break StopReason::PcReached { pc: step.pc };
                    }
                }
                Err(error) => break error.into(),
            }
        };

        let report = RunReport {
            stop,
            attempted_steps,
            completed_steps,
            cycles: self.vm.cpu().cycles(),
            registers: self.vm.cpu().registers(),
            history: history.into_iter().collect(),
        };
        Ok(RunOutcome {
            report,
            vm: self.vm,
        })
    }
}

fn push_history(history: &mut VecDeque<CpuStep>, history_len: usize, step: CpuStep) {
    if history_len == 0 {
        return;
    }
    if history.len() == history_len {
        history.pop_front();
    }
    history.push_back(step);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AddressRange;

    #[derive(Default)]
    struct RecordingHooks {
        events: Vec<String>,
    }

    impl VmRunHooks for RecordingHooks {
        type Error = String;

        fn before_step(&mut self, vm: &mut CompilerVm) -> Result<(), Self::Error> {
            let pc = vm.cpu().registers().pc;
            self.events.push(format!("before:{pc:04X}"));
            if pc == 0x0200 {
                vm.bus_mut().ram_mut().write(0x0040, 0x42);
            }
            Ok(())
        }

        fn after_step(&mut self, vm: &CompilerVm, step: &CpuStep) -> Result<(), Self::Error> {
            self.events.push(format!(
                "after:{:04X}:{:02X}",
                step.pc,
                vm.cpu().registers().a
            ));
            Ok(())
        }
    }

    fn vm_with_program(program: &[u8]) -> CompilerVm {
        let mut vm = CompilerVm::default();
        vm.bus_mut().ram_mut().map(0x0200, program).unwrap();
        vm.set_pc(0x0200);
        vm
    }

    #[test]
    fn stops_at_step_limit_and_retains_bounded_history() {
        let vm = vm_with_program(&[0xEA, 0xEA, 0xEA]);
        let outcome = VmRunner::new(vm).run(RunRequest {
            max_steps: 3,
            history_len: 2,
            ..RunRequest::default()
        });

        assert_eq!(
            outcome.stop_reason(),
            StopReason::StepLimit { max_steps: 3 }
        );
        assert_eq!(outcome.report.attempted_steps, 3);
        assert_eq!(outcome.report.completed_steps, 3);
        assert_eq!(outcome.report.registers.pc, 0x0203);
        assert_eq!(
            outcome
                .report
                .history
                .iter()
                .map(|step| step.pc)
                .collect::<Vec<_>>(),
            vec![0x0201, 0x0202]
        );
    }

    #[test]
    fn stops_after_executing_requested_pc() {
        let vm = vm_with_program(&[0xEA, 0xEA, 0xEA]);
        let outcome = VmRunner::new(vm).run(RunRequest {
            max_steps: 10,
            stop_after_pc: Some(0x0201),
            ..RunRequest::default()
        });

        assert_eq!(outcome.stop_reason(), StopReason::PcReached { pc: 0x0201 });
        assert_eq!(outcome.report.attempted_steps, 2);
        assert_eq!(outcome.report.completed_steps, 2);
        assert_eq!(outcome.report.registers.pc, 0x0202);
    }

    #[test]
    fn reports_unsupported_opcode_and_preserves_final_memory() {
        let mut vm = vm_with_program(&[0xA9, 0x42, 0x85, 0x40, 0x02]);
        vm.bus_mut().ram_mut().write(0x0040, 0x00);
        let outcome = VmRunner::new(vm).run(RunRequest {
            max_steps: 10,
            ..RunRequest::default()
        });

        assert_eq!(
            outcome.stop_reason(),
            StopReason::UnsupportedOpcode {
                pc: 0x0204,
                opcode: 0x02,
            }
        );
        assert_eq!(outcome.report.attempted_steps, 3);
        assert_eq!(outcome.report.completed_steps, 2);
        assert_eq!(outcome.memory().read(0x0040), 0x42);
    }

    #[test]
    fn reports_protected_code_write_without_committing_it() {
        let mut vm = vm_with_program(&[
            0xA9, 0x42, // LDA #$42
            0x8D, 0x05, 0x30, // STA $3005
        ]);
        vm.bus_mut().ram_mut().write(0x3005, 0xEA);
        vm.protect_code_ranges(&[AddressRange {
            start: 0x3000,
            end: 0x30FF,
        }]);
        let outcome = VmRunner::new(vm).run(RunRequest {
            max_steps: 10,
            ..RunRequest::default()
        });

        assert_eq!(
            outcome.stop_reason(),
            StopReason::ProtectedCodeWrite {
                pc: 0x0202,
                address: 0x3005,
                old_value: 0xEA,
                new_value: 0x42,
                region: BusRegion::Ram,
            }
        );
        assert_eq!(outcome.report.attempted_steps, 2);
        assert_eq!(outcome.report.completed_steps, 1);
        assert_eq!(outcome.memory().read(0x3005), 0xEA);
    }

    #[test]
    fn reports_an_already_halted_cpu() {
        let mut vm = vm_with_program(&[0x02]);
        assert!(matches!(
            vm.step_cpu(),
            Err(CpuError::UnsupportedOpcode { .. })
        ));

        let outcome = VmRunner::new(vm).run(RunRequest {
            max_steps: 1,
            ..RunRequest::default()
        });

        assert_eq!(outcome.stop_reason(), StopReason::Halted);
        assert_eq!(outcome.report.attempted_steps, 1);
        assert_eq!(outcome.report.completed_steps, 0);
    }

    #[test]
    fn zero_history_and_zero_step_limit_are_supported() {
        let vm = vm_with_program(&[0xEA]);
        let outcome = VmRunner::new(vm).run(RunRequest {
            max_steps: 0,
            history_len: 0,
            ..RunRequest::default()
        });

        assert_eq!(
            outcome.stop_reason(),
            StopReason::StepLimit { max_steps: 0 }
        );
        assert_eq!(outcome.report.attempted_steps, 0);
        assert_eq!(outcome.report.completed_steps, 0);
        assert!(outcome.report.history.is_empty());
        assert_eq!(outcome.report.registers.pc, 0x0200);
    }

    #[test]
    fn hooks_run_in_instruction_order_and_can_prepare_vm_state() {
        let vm = vm_with_program(&[
            0xA5, 0x40, // LDA $40
            0xEA, // NOP
        ]);
        let mut hooks = RecordingHooks::default();
        let outcome = VmRunner::new(vm)
            .run_with_hooks(
                RunRequest {
                    max_steps: 2,
                    ..RunRequest::default()
                },
                &mut hooks,
            )
            .unwrap();

        assert_eq!(
            hooks.events,
            [
                "before:0200",
                "after:0200:42",
                "before:0202",
                "after:0202:42",
            ]
        );
        assert_eq!(outcome.report.completed_steps, 2);
        assert_eq!(outcome.report.history.len(), 2);
    }

    #[test]
    fn post_step_hook_is_not_called_when_the_cpu_step_fails() {
        let vm = vm_with_program(&[0x02]);
        let mut hooks = RecordingHooks::default();
        let outcome = VmRunner::new(vm)
            .run_with_hooks(
                RunRequest {
                    max_steps: 1,
                    ..RunRequest::default()
                },
                &mut hooks,
            )
            .unwrap();

        assert_eq!(hooks.events, ["before:0200"]);
        assert!(matches!(
            outcome.stop_reason(),
            StopReason::UnsupportedOpcode { .. }
        ));
    }
}
