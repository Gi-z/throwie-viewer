use crate::csi::{CSIMeasurement, get_scaling_factor};

pub fn get_initial_matrix(readings: &Vec<CSIMeasurement>) -> ([[f32; 64]; 100], f32) {
    let mut matrix = [[0 as f32; 64]; 100];
    let mut maxval: f32 = 0.0;

    for (i, vec) in readings.iter().enumerate() {
        // first we need to convert the complex numbers to magnitude
        let mag_vals = vec.csi_amp;
        let scale = get_scaling_factor(&mag_vals, vec.rssi); // RSSI-based rescaling factor

        for j in 0..64 {
            let db_val = 20 as f32 * mag_vals[j].log10();
            let scaled_val = db_val * scale.sqrt();

            matrix[i][j] = scaled_val;

            if scaled_val > maxval {
                maxval = scaled_val;
            }
        }
    }

    (matrix, maxval)
}

pub fn update_matrix(matrix: [[f32; 64]; 100], mut maxval: f32, reading: &CSIMeasurement) -> ([[f32; 64]; 100], f32) {
    // println!("Processing timestamp: {}", reading.time);

    let mut newline = [0 as f32; 64];

    let mag_vals = reading.csi_amp;
    let scale = get_scaling_factor(&mag_vals, reading.rssi); // RSSI-based rescaling factor

    for j in 0..64 {
        let db_val = 20 as f32 * mag_vals[j].log10();
        let scaled_val = db_val * scale.sqrt();

        newline[j] = scaled_val;

        if scaled_val > maxval {
            maxval = scaled_val;
        }
    }

    let mut newmatrix = [[0_f32; 64]; 100];

    for i in 0..100 {
        if i != 0 {
            newmatrix[i - 1] = matrix[i];
        }
    }
    newmatrix[99] = newline;

    (newmatrix, maxval)
}