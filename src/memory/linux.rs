use std::fs;
use std::io::IoSliceMut;

use nix::errno::Errno;
use nix::sys::uio::{RemoteIoVec, process_vm_readv};
use nix::unistd::Pid;

use crate::memory::process::{ Process, MemoryRegion, ProcessTraits };
use crate::memory::error::ProcessError;

use super::signature::find_signature;
use super::signature::Signature;

impl ProcessTraits for Process {
    fn initialize(
        proc_name: &str
    ) -> Result<Process, super::error::ProcessError> {
        let process = Process::find_process(proc_name)?;
        process.read_regions()
    }

    fn find_process(proc_name: &str) -> Result<Process, ProcessError> {
        let mut found: bool = false;

        let paths = fs::read_dir("/proc")?;

        let mut pid: i32 = -1;
        for path in paths {
            
            let p = path?.path();

            if !p.is_dir() {
                continue;
            }

            let cmd_line = p.join("cmdline");

            if !cmd_line.exists() {
                continue;
            }

            let buff = fs::read_to_string(cmd_line)?;
            let line = buff.split(' ').next().unwrap();

            if line.contains(proc_name) {
                let stat = p.join("stat");
                let buff = fs::read_to_string(stat)?;

                let pid_str = buff.split(' ').next().unwrap();
                
                pid = pid_str.parse()?;
                found = true;
                break;
            }
        }

        match found {
            true => Ok(Self { pid, maps: Vec::new() }),
            false => Err(ProcessError::ProcessNotFound)
        }
    }

    fn read_regions(mut self) -> Result<Process, ProcessError> {
        let path = format!("/proc/{}/maps", &self.pid);
        let mut v = Vec::new();
        
        let buff = fs::read_to_string(&path)?;
    
        for line in buff.split('\n')
        {
            if line.is_empty() {
                break;
            }
    
            let mut split = line.split_whitespace();
            let range_raw = split.next().unwrap();
            let mut range_split = range_raw.split('-');

            let from_str = range_split.next().unwrap();
            let to_str = range_split.next().unwrap();

            let from = usize::from_str_radix(
                from_str, 16
            )?;

            let to = usize::from_str_radix(
                to_str, 16
            )?;
    
            v.push(MemoryRegion{ from, size: to - from });
        }
    
        self.maps = v;
        Ok(self)
    }

    fn read_signature(
        &self, 
        sign: &Signature
    ) -> Result<usize, ProcessError> {
        for region in &self.maps {
            let remote = RemoteIoVec {
                base: region.from,
                len: region.size
            };

            let mut buff = vec![0; region.size];

            let slice = IoSliceMut::new(buff.as_mut_slice());

            let res = process_vm_readv(
                Pid::from_raw(self.pid),
                &mut [slice],
                &[remote]
            );
            
            if let Err(e) = res {
                match e {
                    Errno::EPERM | Errno::ESRCH =>
                        return Err(e.into()),
                    _ => continue,
                }
            }

            let res = find_signature(buff.as_slice(), sign);
            if let Some(offset) = res {
                return Ok(remote.base + offset);
            }
        }

        Err(ProcessError::SignatureNotFound(sign.to_string()))
    }

    fn read(
        &self, 
        addr: usize, 
        len: usize, 
        buff: &mut [u8]
    ) -> Result<(), ProcessError> {
        let remote = RemoteIoVec {
            base: addr,
            len
        };

        let slice = IoSliceMut::new(buff);

        let res = process_vm_readv(
            Pid::from_raw(self.pid),
            &mut [slice],
            &[remote]
        );

        match res {
            Ok(_) => (),
            Err(e) => {
                match e {
                    nix::errno::Errno::EFAULT => 
                        return Err(ProcessError::BadAddress(addr, len)),
                    _ => return Err(e.into()),
                }
            },
        }

        Ok(())
    }
}
