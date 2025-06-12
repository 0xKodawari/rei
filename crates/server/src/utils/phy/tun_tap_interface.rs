extern crate alloc;
use alloc::rc::Rc;
use libc::{c_char, c_int, close};
use core::{cell::RefCell, result::Result};
use super::linux::*;
use crate::utils::errors::Error;

#[derive(Debug, Clone, Copy)]
pub enum Medium {
    Ethernet,
    Ip,
    Ieee802154
}

#[repr(C)]
#[derive(Debug)]
struct Ifreq {
    ifr_name: [libc::c_char; libc::IF_NAMESIZE],
    ifr_data: libc::c_int, /* ifr_ifindex or ifr_mtu */
}

struct TunTapInterfaceDesc {
    lower: c_int,
    mtu: usize
}



impl TunTapInterfaceDesc {
    fn new(name: &str, medium: Medium) -> Result<TunTapInterfaceDesc, Error> {
        let lower = unsafe {
            let lower = libc::open(
                "/dev/net/tun\0".as_ptr() as *const libc::c_char,
                libc::O_RDWR | libc::O_NONBLOCK,
            );
            if lower == -1 {
                //return Err(io::Error::last_os_error());
            }
            lower
        };
        let mut ifreq = ifreq_for(name);
        Self::attach_interface_ifreq(lower, medium, &mut ifreq)?;
        let mtu = Self::mtu_ifreq(medium, &mut ifreq)?;

        Ok(TunTapInterfaceDesc { lower, mtu})
    } 

    fn attach_interface_ifreq(
        lower: libc::c_int,
        medium: Medium,
        ifr: &mut Ifreq,
    ) -> Result<(), Error> {
        let mode = match medium {
          
            Medium::Ip => IFF_TUN,
          
            Medium::Ethernet => IFF_TAP,
          
            Medium::Ieee802154 => todo!(),
        };
        ifr.ifr_data = mode | IFF_NO_PI;
        ifreq_ioctl(lower, ifr, TUNSETIFF).map(|_| ())
    }

    fn mtu_ifreq(medium: Medium, ifr: &mut Ifreq) -> Result<usize, Error> {
        let lower = unsafe {
            let lower = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, libc::IPPROTO_IP);
            if lower == -1 {
                //return Err(io::Error::last_os_error());
            }
            lower
        };

        let ip_mtu = ifreq_ioctl(lower, ifr, SIOCGIFMTU).map(|mtu| mtu as usize);

        unsafe {
            libc::close(lower);
        }

        // Propagate error after close, to ensure we always close.
        let ip_mtu = ip_mtu?;

        // SIOCGIFMTU returns the IP MTU (typically 1500 bytes.)
        // smoltcp counts the entire Ethernet packet in the MTU, so add the Ethernet header size to it.
        let mtu = match medium {
            Medium::Ip => ip_mtu,
            Medium::Ethernet => ip_mtu, //Add header length back to this 
            Medium::Ieee802154 => todo!(),
        };

        Ok(mtu)
    }

    pub fn interface_mtu(&self) -> Result<usize, Error> {
        Ok(self.mtu)
    }
}

impl Drop for TunTapInterfaceDesc {
    fn drop(&mut self) {
        unsafe {
            close(self.lower);
        }
    }
}

fn ifreq_for(name: &str) -> Ifreq {
    let mut ifreq = Ifreq {
        ifr_name: [0; libc::IF_NAMESIZE],
        ifr_data: 0,
    };
    for (i, byte) in name.as_bytes().iter().enumerate() {
        ifreq.ifr_name[i] = *byte as c_char
    }
    ifreq
}


fn ifreq_ioctl(
    lower: libc::c_int,
    ifreq: &mut Ifreq,
    cmd: libc::c_ulong,
) -> Result<c_int, Error> {
    unsafe {
        let res = libc::ioctl(lower, cmd as _, ifreq as *mut Ifreq);
        if res == -1 {
            //return Err(io::Error::last_os_error());
        }
    }

    Ok(ifreq.ifr_data)
}

pub struct TunTapInterface {
    lower: Rc<RefCell<TunTapInterfaceDesc>>,
    mtu: usize,
    medium: Medium,
}

impl TunTapInterface {
    pub fn new(name: &str, medium: Medium) -> Result<TunTapInterface, Error> {
        let lower = TunTapInterfaceDesc::new(name, medium)?;
        let mtu = lower.interface_mtu()?;
        Ok(TunTapInterface {
            lower: Rc::new(RefCell::new(lower)),
            mtu,
            medium,
        })
    }
}
