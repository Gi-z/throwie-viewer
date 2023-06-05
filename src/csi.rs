use std::net::UdpSocket;

use thiserror::Error;

use protobuf::{Message};

include!(concat!(env!("OUT_DIR"), "/proto/mod.rs"));
use csimsg::{CSIMessage};

pub const UDP_SERVER_PORT: u16 = 6969;
pub const UDP_MESSAGE_SIZE: usize = 170;

const CSI_METRICS_MEASUREMENT: &str = "csi_metrics";

#[derive(Error, Debug)]
pub enum RetrieveCSIError {
    #[error("Could not receive data from UDP socket.")]
    SocketRecvError(),

    #[error("CSI expected_size: {0} is larger than allocated buffer: {1}.")]
    CSITooBigError(usize, usize),

    #[error("Failed to parse protobuf from buffer contents.")]
    ProtobufParseError(#[from] protobuf::Error),
}

#[derive(Copy, Clone)]
pub struct CSIMeasurement {
    pub time: u128,
    pub rssi: i8,
    pub csi_amp: [f32; 64]
}

pub fn open_csi_socket() -> UdpSocket {
    let socket_result = UdpSocket::bind(("0.0.0.0", UDP_SERVER_PORT));
    match socket_result {
        Ok(sock) => sock,
        Err(error) => panic!("Encountered error when opening port {:?}: {:?}", UDP_SERVER_PORT, error)
    }
}

pub fn recv_message(socket: &UdpSocket) -> Result<CSIMessage, RetrieveCSIError> {
    let mut buf = [0; UDP_MESSAGE_SIZE];
    let recv_result = socket.recv_from(&mut buf);
    let (_, _) = match recv_result {
        Ok(i) => i,
        Err(_) => return Err(RetrieveCSIError::SocketRecvError())
    };

    let expected_size = buf[0] as usize;
    
    // If the size we expect to read is too large for buf then throw error.
    if expected_size > UDP_MESSAGE_SIZE - 1 {
        return Err(RetrieveCSIError::CSITooBigError(expected_size, UDP_MESSAGE_SIZE))
    }

    let expected_protobuf = &buf[1 .. expected_size + 1];
    let msg = CSIMessage::parse_from_bytes(expected_protobuf)?;

    Ok(msg)
}

pub fn get_csi_measurement(msg: &CSIMessage) -> CSIMeasurement {
    let rssi = i8::try_from(msg.rssi.unwrap()).ok().unwrap();

    let timestamp_us = u128::try_from(msg.timestamp.unwrap()).unwrap();
    // println!("Processing timestamp: {}", timestamp_us);
    // let timestamp = Timestamp::Microseconds(timestamp_us).into();

    // let src_mac = format!("0x{:X}", msg.src_mac.clone().unwrap()[5]);

    let csi_bytes = msg.csi_data.clone().unwrap();
    let csi_ints: Vec<i8> = csi_bytes.into_iter().map(|v| v as i8).collect();
    let mut csi_array: [f32; 64] = [0.0; 64];
    let mut ij = 0;
    
    for i in (0..128).step_by(2) {
        let real = csi_ints[i];
        let imag = csi_ints[i + 1];
        // println!("Instantiating complex number with components real: {} and imag: {}", real, imag);
        let compl = num::Complex::new(real as f32, imag as f32);
        let abs = compl.norm();

        csi_array[ij] = abs;

        ij += 1;
    }

    // println!("Array {:?}", csi_array);
    
    CSIMeasurement {
        time: timestamp_us,
        rssi: rssi,
        csi_amp: csi_array
    }
}

pub fn get_scaling_factor(mag_vals: &[f32; 64], rssi: i8) -> f32 {
    let rssi_pwr = 10_f32.powi(rssi as i32 / 10);
    // println!("Scaling CSIMeasurement CSI with RSSI_pwr {:?}", rssi_pwr);
    let vec_mag = mag_vals.iter().map(|x| x.powi(2)).sum::<f32>();
    let norm_vec_mag = vec_mag / 64_f32;
    
    rssi_pwr / norm_vec_mag
}