//! SPI Commands for the Waveshare 2.9" (B/C) E-Ink Display
use crate::traits;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum Command {
    DriverOutputControl = 0x01,
    GateDrivingVoltageCtrl = 0x03,
    SourceDrivingVoltageCtrl = 0x04,
    BoosterSoftStartControl = 0x0C,
    GateScanStartPosition = 0x0F,
    DeepSleepMode = 0x10,
    DataEntryModeSetting = 0x11,
    SwReset = 0x12,
    HvReadyDetection = 0x14,
    VciDetection = 0x15,
    TemperatureSensorControlWrite = 0x1A,
    TemperatureSensorControlRead = 0x1B,
    TemperatureSensorExtControlWrite = 0x1C,
    MasterActivation = 0x20,
    DisplayUpdateControl1 = 0x21,
    DisplayUpdateControl2 = 0x22,
    WriteRam = 0x24,
    WriteRamRed = 0x26,
    ReadRam = 0x27,
    VcomSense = 0x28,
    VcomSenseDuration = 0x29,
    ProgramVcomOpt = 0x2A,
    WriteVcomRegister = 0x2C,
    OtpRegisterRead = 0x2D,
    StatusBitRead = 0x2F,
    ProgramWsOtp = 0x30,
    LoadWsOtp = 0x31,
    WriteLutRegister = 0x32,
    ProgramOtpSelection = 0x36,
    WriteOtpSelection = 0x37,
    SetDummyLinePeriod = 0x3A,
    SetGateLineWidth = 0x3B,
    BorderWaveformControl = 0x3C,
    ReadRamOption = 0x41,
    SetRamXAddressStartEndPosition = 0x44,
    SetRamYAddressStartEndPosition = 0x45,
    AutoWriteRedRamRegularPattern = 0x46,
    AutoWriteBwRamRegularPattern = 0x47,
    SetRamXAddressCounter = 0x4E,
    SetRamYAddressCounter = 0x4F,
    SetAnalogBlockControl = 0x74,
    SetDigitalBlockControl = 0x7E,

    Nop = 0x7F,
    TerminateFrameReadWrite = 0xFF,
}

impl traits::Command for Command {
    /// Returns the address of the command
    fn address(self) -> u8 {
        self as u8
    }
}
