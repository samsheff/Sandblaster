use sandblaster_core::InstructionBytes;

const X86_64_BITNESS: u32 = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodeOutput {
    pub mnemonic: String,
    pub operands: String,
    pub length: u32,
    pub known: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodeError {
    pub message: String,
}

pub trait DisasmBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn decode_first(&self, instruction: &InstructionBytes) -> Result<DecodeOutput, DecodeError>;
}

#[derive(Clone, Debug, Default)]
pub struct NullDisassembler;

impl DisasmBackend for NullDisassembler {
    fn name(&self) -> &'static str {
        "null"
    }

    fn decode_first(&self, _instruction: &InstructionBytes) -> Result<DecodeOutput, DecodeError> {
        Ok(DecodeOutput {
            mnemonic: "(unk)".to_string(),
            operands: String::new(),
            length: 0,
            known: false,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct IcedX86Disassembler;

impl DisasmBackend for IcedX86Disassembler {
    fn name(&self) -> &'static str {
        "iced-x86"
    }

    fn decode_first(&self, instruction: &InstructionBytes) -> Result<DecodeOutput, DecodeError> {
        let bytes = &instruction.bytes()[..sandblaster_core::MAX_INSN_LENGTH];
        let mut decoder =
            iced_x86::Decoder::new(X86_64_BITNESS, bytes, iced_x86::DecoderOptions::NONE);
        let decoded = decoder.decode();

        if decoded.is_invalid() {
            return Ok(DecodeOutput {
                mnemonic: "(unk)".to_string(),
                operands: String::new(),
                length: 0,
                known: false,
            });
        }

        Ok(DecodeOutput {
            mnemonic: format!("{:?}", decoded.mnemonic()).to_ascii_lowercase(),
            operands: String::new(),
            length: decoded.len() as u32,
            known: true,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Arm64FixedDisassembler;

impl DisasmBackend for Arm64FixedDisassembler {
    fn name(&self) -> &'static str {
        "arm64-fixed"
    }

    fn decode_first(&self, instruction: &InstructionBytes) -> Result<DecodeOutput, DecodeError> {
        if instruction.specified_len() < 4 {
            return Ok(DecodeOutput {
                mnemonic: "(short)".to_string(),
                operands: String::new(),
                length: 0,
                known: false,
            });
        }

        Ok(DecodeOutput {
            mnemonic: "aarch64".to_string(),
            operands: String::new(),
            length: 4,
            known: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use sandblaster_core::InstructionBytes;

    use crate::backend::{Arm64FixedDisassembler, DisasmBackend, IcedX86Disassembler};

    #[test]
    fn iced_decodes_known_instruction_length() {
        let decoded = IcedX86Disassembler
            .decode_first(&InstructionBytes::from_slice(&[0x90]))
            .expect("decode should succeed");

        assert!(decoded.known);
        assert_eq!(decoded.length, 1);
    }

    #[test]
    fn iced_reports_unknown_instruction_like_capstone_raw_path() {
        let decoded = IcedX86Disassembler
            .decode_first(&InstructionBytes::from_slice(&[0x82]))
            .expect("decode should succeed");

        assert!(!decoded.known);
        assert_eq!(decoded.length, 0);
    }

    #[test]
    fn arm64_fixed_width_reports_four_byte_instructions() {
        let decoded = Arm64FixedDisassembler
            .decode_first(&InstructionBytes::from_slice(&[0x1f, 0x20, 0x03, 0xd5]))
            .expect("decode should succeed");

        assert!(decoded.known);
        assert_eq!(decoded.length, 4);
    }
}
