use libc::c_int;

const MAX_NEURONS: usize = 128;

const TANSIG_TABLE: [f32; 201] = [
    0.000000, 0.039979, 0.079830, 0.119427, 0.158649, 0.197375, 0.235496, 0.272905, 0.309507,
    0.345214, 0.379949, 0.413644, 0.446244, 0.477700, 0.507977, 0.537050, 0.564900, 0.591519,
    0.616909, 0.641077, 0.664037, 0.685809, 0.706419, 0.725897, 0.744277, 0.761594, 0.777888,
    0.793199, 0.807569, 0.821040, 0.833655, 0.845456, 0.856485, 0.866784, 0.876393, 0.885352,
    0.893698, 0.901468, 0.908698, 0.915420, 0.921669, 0.927473, 0.932862, 0.937863, 0.942503,
    0.946806, 0.950795, 0.954492, 0.957917, 0.961090, 0.964028, 0.966747, 0.969265, 0.971594,
    0.973749, 0.975743, 0.977587, 0.979293, 0.980869, 0.982327, 0.983675, 0.984921, 0.986072,
    0.987136, 0.988119, 0.989027, 0.989867, 0.990642, 0.991359, 0.992020, 0.992631, 0.993196,
    0.993718, 0.994199, 0.994644, 0.995055, 0.995434, 0.995784, 0.996108, 0.996407, 0.996682,
    0.996937, 0.997172, 0.997389, 0.997590, 0.997775, 0.997946, 0.998104, 0.998249, 0.998384,
    0.998508, 0.998623, 0.998728, 0.998826, 0.998916, 0.999000, 0.999076, 0.999147, 0.999213,
    0.999273, 0.999329, 0.999381, 0.999428, 0.999472, 0.999513, 0.999550, 0.999585, 0.999617,
    0.999646, 0.999673, 0.999699, 0.999722, 0.999743, 0.999763, 0.999781, 0.999798, 0.999813,
    0.999828, 0.999841, 0.999853, 0.999865, 0.999875, 0.999885, 0.999893, 0.999902, 0.999909,
    0.999916, 0.999923, 0.999929, 0.999934, 0.999939, 0.999944, 0.999948, 0.999952, 0.999956,
    0.999959, 0.999962, 0.999965, 0.999968, 0.999970, 0.999973, 0.999975, 0.999977, 0.999978,
    0.999980, 0.999982, 0.999983, 0.999984, 0.999986, 0.999987, 0.999988, 0.999989, 0.999990,
    0.999990, 0.999991, 0.999992, 0.999992, 0.999993, 0.999994, 0.999994, 0.999994, 0.999995,
    0.999995, 0.999996, 0.999996, 0.999996, 0.999997, 0.999997, 0.999997, 0.999997, 0.999997,
    0.999998, 0.999998, 0.999998, 0.999998, 0.999998, 0.999998, 0.999999, 0.999999, 0.999999,
    0.999999, 0.999999, 0.999999, 0.999999, 0.999999, 0.999999, 0.999999, 0.999999, 0.999999,
    0.999999, 1.000000, 1.000000, 1.000000, 1.000000, 1.000000, 1.000000, 1.000000, 1.000000,
    1.000000, 1.000000, 1.000000,
];

fn tansig_approx(x: f32) -> f32 {
    // Tests are reversed to catch NaNs
    if !(x < 8.0) {
        return 1.0;
    }
    if !(x > -8.0) {
        return -1.0;
    }

    let (mut x, sign) = if x < 0.0 { (-x, -1.0) } else { (x, 1.0) };
    let i = (0.5 + 25.0 * x).floor();
    x -= 0.04 * i;
    let y = TANSIG_TABLE[i as usize];
    let dy = 1.0 - y * y;
    let y = y + x * dy * (1.0 - y * x);
    sign * y
}

fn sigmoid_approx(x: f32) -> f32 {
    0.5 + 0.5 * tansig_approx(0.5 * x)
}

fn relu(x: f32) -> f32 {
    x.max(0.0)
}

#[repr(u8)]
pub enum Activation {
    Tanh = 0,
    Sigmoid = 1,
    Relu = 2,
}

const WEIGHTS_SCALE: f32 = 1.0 / 256.0;

#[repr(C)]
pub struct DenseLayer {
    bias: *const i8,
    input_weights: *const i8,
    nb_inputs: c_int,
    nb_neurons: c_int,
    activation: c_int,
}

#[repr(C)]
pub struct GruLayer {
    bias: *const i8,
    input_weights: *const i8,
    recurrent_weights: *const i8,
    nb_inputs: c_int,
    nb_neurons: c_int,
    activation: c_int,
}

#[no_mangle]
pub extern "C" fn compute_dense(layer: *const DenseLayer, output: *mut f32, input: *const f32) {
    unsafe {
        let layer = &*layer;
        let output_slice = std::slice::from_raw_parts_mut(output, layer.nb_neurons as usize);
        let input_slice = std::slice::from_raw_parts(input, layer.nb_inputs as usize);
        rs_compute_dense(layer, output_slice, input_slice);
    }
}

fn rs_compute_dense(layer: &DenseLayer, output: &mut [f32], input: &[f32]) {
    let m = layer.nb_inputs as isize;
    let n = layer.nb_neurons as isize;
    let stride = n;

    for i in 0..n {
        // Compute update gate.
        let mut sum = unsafe { *layer.bias.offset(i) } as f32;
        for j in 0..m {
            sum +=
                unsafe { *layer.input_weights.offset(j * stride + i) } as f32 * input[j as usize];
        }
        output[i as usize] = WEIGHTS_SCALE * sum;
    }
    if layer.activation == Activation::Sigmoid as c_int {
        for i in 0..n as usize {
            output[i] = sigmoid_approx(output[i]);
        }
    } else if layer.activation == Activation::Tanh as c_int {
        for i in 0..n as usize {
            output[i] = tansig_approx(output[i]);
        }
    } else if layer.activation == Activation::Relu as c_int {
        for i in 0..n as usize {
            output[i] = relu(output[i]);
        }
    } else {
        panic!("bad activation");
    }
}

#[no_mangle]
pub extern "C" fn compute_gru(gru: *const GruLayer, state: *mut f32, input: *const f32) {
    unsafe {
        let gru = &*gru;
        let state_slice = std::slice::from_raw_parts_mut(state, gru.nb_neurons as usize);
        let input_slice = std::slice::from_raw_parts(input, gru.nb_inputs as usize);
        rs_compute_gru(gru, state_slice, input_slice);
    }
}

fn rs_compute_gru(gru: &GruLayer, state: &mut [f32], input: &[f32]) {
    let mut z = [0.0; MAX_NEURONS];
    let mut r = [0.0; MAX_NEURONS];
    let mut h = [0.0; MAX_NEURONS];
    let m = gru.nb_inputs as isize;
    let n = gru.nb_neurons as isize;
    let stride = 3 * n;

    for i in 0..n {
        // Compute update gate.
        let mut sum = unsafe { *gru.bias.offset(i) } as f32;
        for j in 0..m {
            sum += unsafe { *gru.input_weights.offset(j * stride + i) } as f32 * input[j as usize];
        }
        for j in 0..n {
            sum +=
                unsafe { *gru.recurrent_weights.offset(j * stride + i) } as f32 * state[j as usize];
        }
        z[i as usize] = sigmoid_approx(WEIGHTS_SCALE * sum);
    }
    for i in 0..n {
        // Compute reset gate.
        let mut sum = unsafe { *gru.bias.offset(n + i) } as f32;
        for j in 0..m {
            sum +=
                unsafe { *gru.input_weights.offset(n + j * stride + i) } as f32 * input[j as usize];
        }
        for j in 0..n {
            sum += unsafe { *gru.recurrent_weights.offset(n + j * stride + i) } as f32
                * state[j as usize];
        }
        r[i as usize] = sigmoid_approx(WEIGHTS_SCALE * sum);
    }
    for i in 0..n {
        // Compute output.
        let mut sum = unsafe { *gru.bias.offset(2 * n + i) } as f32;
        for j in 0..m {
            sum += unsafe { *gru.input_weights.offset(2 * n + j * stride + i) } as f32
                * input[j as usize];
        }
        for j in 0..n {
            sum += unsafe { *gru.recurrent_weights.offset(2 * n + j * stride + i) } as f32
                * state[j as usize]
                * r[j as usize];
        }
        let sum = if gru.activation == Activation::Sigmoid as c_int {
            sigmoid_approx(WEIGHTS_SCALE * sum)
        } else if gru.activation == Activation::Tanh as c_int {
            tansig_approx(WEIGHTS_SCALE * sum)
        } else if gru.activation == Activation::Relu as c_int {
            relu(WEIGHTS_SCALE * sum)
        } else {
            panic!("bad activation")
        };
        let i = i as usize;
        h[i] = z[i] * state[i] + (1.0 - z[i]) * sum;
    }
    for i in 0..n as usize {
        state[i] = h[i];
    }
}
