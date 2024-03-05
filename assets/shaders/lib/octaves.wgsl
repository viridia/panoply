fn sum_octaves_8f(
    level: ptr<private, array<f32, 8>>,
    start_octave: u32,
    end_octave: u32,
    persistence: f32) -> f32 {
  var coeff: f32 = 1.0;
  var accum: f32 = 0.0;
  var total: f32 = 0.0;
  for (var i = 0u; i < 8u; i++) {
    if (i >= start_octave && i <= end_octave) {
      accum += (*level)[i] * coeff;
      total += coeff;
      coeff *= persistence;
    }
  }
  return accum / total;
}

fn sum_octaves_6f(
    level: ptr<private, array<f32, 6>>,
    start_octave: u32,
    end_octave: u32,
    persistence: f32) -> f32 {
  var coeff: f32 = 1.0;
  var accum: f32 = 0.0;
  var total: f32 = 0.0;
  for (var i = 0u; i < 6u; i++) {
    if (i >= start_octave && i <= end_octave) {
      accum += (*level)[i] * coeff;
      total += coeff;
      coeff *= persistence;
    }
  }
  return accum / total;
}

fn sum_octaves_8v2f(
    level: ptr<private, array<vec2<f32>, 8>>,
    start_octave: u32,
    end_octave: u32,
    persistence: f32) -> vec2<f32> {
  var coeff: f32 = 1.0;
  var accum = vec2<f32>(0., 0.);
  var total: f32 = 0.0;
  for (var i = 0u; i < 8u; i++) {
    if (i >= start_octave && i <= end_octave) {
      accum += (*level)[i] * coeff;
      total += coeff;
      coeff *= persistence;
    }
  }
  return accum / total;
}

fn sum_octaves_6v2f(
    level: ptr<function, array<vec2<f32>, 6>>,
    start_octave: u32,
    end_octave: u32,
    persistence: f32) -> vec2<f32> {
  var coeff: f32 = 1.0;
  var accum = vec2<f32>(0., 0.);
  var total: f32 = 0.0;
  for (var i = 0u; i < 6u; i++) {
    if (i >= start_octave && i <= end_octave) {
      accum += (*level)[i] * coeff;
      total += coeff;
      coeff *= persistence;
    }
  }
  return accum / total;
}

fn sum_octaves_8v3f(
    level: ptr<function, array<vec3<f32>, 8>>,
    start_octave: u32,
    end_octave: u32,
    persistence: f32) -> vec3<f32> {
  var coeff: f32 = 1.0;
  var accum = vec3<f32>(0., 0., 0.);
  var total: f32 = 0.0;
  for (var i = 0u; i < 8u; i++) {
    if (i >= start_octave && i <= end_octave) {
      accum += (*level)[i] * coeff;
      total += coeff;
      coeff *= persistence;
    }
  }
  return accum / total;
}

fn sum_octaves_6v3f(
    level: ptr<function, array<vec3<f32>, 6>>,
    start_octave: u32,
    end_octave: u32,
    persistence: f32) -> vec3<f32> {
  var coeff: f32 = 1.0;
  var accum = vec3<f32>(0., 0., 0.);
  var total: f32 = 0.0;
  for (var i = 0u; i < 6u; i++) {
    if (i >= start_octave && i <= end_octave) {
      accum += (*level)[i] * coeff;
      total += coeff;
      coeff *= persistence;
    }
  }
  return accum / total;
}
