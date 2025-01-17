use super::error::ProcessError;
use super::signature::Signature;
use paste::paste;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[derive(Debug)]
pub struct MemoryRegion {
    pub from: usize,
    pub size: usize
}

macro_rules! prim_read_impl {
    ($t: ident) => {
        paste! {
            fn [<read_ $t>](
                &self,
                addr: usize
            ) -> Result<$t, ProcessError> {
                let mut bytes = [0u8; std::mem::size_of::<$t>()];
                self.read(addr, std::mem::size_of::<$t>(), &mut bytes)?;


                Ok($t::from_le_bytes(bytes))
            }
        }
    }
}

pub struct Process {
    #[cfg(target_os = "linux")]
    pub pid: i32,

    #[cfg(target_os = "windows")]
    pub pid: u32,

    #[cfg(target_os = "windows")]
    pub handle: HANDLE,

    pub maps: Vec<MemoryRegion>,
}

pub trait ProcessTraits where Self: Sized {
    fn initialize(proc_name: &str) -> Result<Self, ProcessError>;
    fn find_process(proc_name: &str) -> Result<Self, ProcessError>;
    fn read_regions(self) -> Result<Self, ProcessError>;

    fn read_signature(
        &self, 
        sign: &Signature
    ) -> Result<usize, ProcessError>;

    fn read(
        &self, 
        addr: usize, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError>;

    fn read_uleb128(
        &self,
        mut addr: usize
    ) -> Result<u64, ProcessError> {
        let mut value: u64 = 0;
        let mut bytes_read = 0;

        loop {
            let byte = self.read_u8(addr)?;
            addr += 1;

            let byte_value = (byte & 0b0111_1111) as u64;
            value |= byte_value << (7 * bytes_read);

            bytes_read += 1;

            if (byte &!0b0111_1111) == 0 {
                break;
            }
        }

        Ok(value)
    }

    fn read_string(
        &self,
        mut addr: usize
    ) -> Result<String, ProcessError> {
        let len = self.read_u32(addr + 0x4)? * 2;
        addr += 0x8;

        let mut buff = vec![0u8; len as usize];
        self.read(addr, len as usize, &mut buff)?;

        // TODO check align of bytes
        let buff = unsafe { 
            std::slice::from_raw_parts(
                buff.as_ptr() as *const u16, 
                (len / 2) as usize
            )
        };

        Ok(String::from_utf16_lossy(buff))
    }

    prim_read_impl!(i8);
    prim_read_impl!(i16);
    prim_read_impl!(i32);
    prim_read_impl!(i64);
    prim_read_impl!(i128);

    prim_read_impl!(u8);
    prim_read_impl!(u16);
    prim_read_impl!(u32);
    prim_read_impl!(u64);
    prim_read_impl!(u128);

    prim_read_impl!(f32);
    prim_read_impl!(f64);
}

