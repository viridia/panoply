#import "shaders/permute.wgsl" mod289_v3f, permute_v4f, taylorInvSqrt

//  MIT License. © Ian McEwan
fn snoise_grad(v: vec3<f32>, gradient: ptr<function, vec3<f32>>) -> f32 {
  let C = vec2<f32>(1.0 / 6.0, 1.0 / 3.0);
  let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);

  // First corner
  var i = floor(v + dot(v, C.yyy));
  let x0 = v - i + dot(i, C.xxx) ;

  // Other corners
  let g = step(x0.yzx, x0.xyz);
  let l = 1.0 - g;
  let i1 = min(g.xyz, l.zxy);
  let i2 = max(g.xyz, l.zxy);

  //   x0 = x0 - 0.0 + 0.0 * C.xxx;
  //   x1 = x0 - i1  + 1.0 * C.xxx;
  //   x2 = x0 - i2  + 2.0 * C.xxx;
  //   x3 = x0 - 1.0 + 3.0 * C.xxx;
  let x1 = x0 - i1 + C.xxx;
  let x2 = x0 - i2 + C.yyy; // 2.0*C.x = 1/3 = C.y
  let x3 = x0 - D.yyy;      // -1.0+3.0*C.x = -0.5 = -D.y

//   // Permutations
  i = mod289_v3f(i);
  let p: vec4<f32> = permute_v4f(permute_v4f(permute_v4f(
        i.z + vec4(0.0, i1.z, i2.z, 1.0 ))
      + i.y + vec4(0.0, i1.y, i2.y, 1.0 ))
      + i.x + vec4(0.0, i1.x, i2.x, 1.0 ));

  // Gradients: 7x7 points over a square, mapped onto an octahedron.
  // The ring size 17*17 = 289 is close to a multiple of 49 (49*6 = 294)
  let n_ = 0.142857142857; // 1.0/7.0
  let ns = n_ * D.wyz - D.xzx;

  let j: vec4<f32> = p - 49.0 * floor(p * ns.z * ns.z);  //  mod(p,7*7)

  let x_: vec4<f32> = floor(j * ns.z);
  let y_: vec4<f32> = floor(j - 7.0 * x_);    // mod(j,N)

  let x: vec4<f32> = x_ *ns.x + ns.yyyy;
  let y: vec4<f32> = y_ *ns.x + ns.yyyy;
  let h: vec4<f32> = 1.0 - abs(x) - abs(y);

  let b0: vec4<f32> = vec4(x.xy, y.xy);
  let b1: vec4<f32> = vec4(x.zw, y.zw);

  //vec4 s0 = vec4(lessThan(b0,0.0))*2.0 - 1.0;
  //vec4 s1 = vec4(lessThan(b1,0.0))*2.0 - 1.0;
  let s0: vec4<f32> = floor(b0) * 2.0 + 1.0;
  let s1: vec4<f32> = floor(b1) * 2.0 + 1.0;
  let sh: vec4<f32> = -step(h, vec4(0.0));

  let a0: vec4<f32> = b0.xzyw + s0.xzyw * sh.xxyy ;
  let a1: vec4<f32> = b1.xzyw + s1.xzyw * sh.zzww ;

  var p0: vec3<f32> = vec3(a0.xy, h.x);
  var p1: vec3<f32> = vec3(a0.zw, h.y);
  var p2: vec3<f32> = vec3(a1.xy, h.z);
  var p3: vec3<f32> = vec3(a1.zw, h.w);

  // Normalise gradients
  let norm: vec4<f32> = taylorInvSqrt(vec4(dot(p0,p0), dot(p1,p1), dot(p2, p2), dot(p3,p3)));
  p0 *= norm.x;
  p1 *= norm.y;
  p2 *= norm.z;
  p3 *= norm.w;

  // Mix final noise value
  let m: vec4<f32> = max(
    0.5 - vec4(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)),
    vec4<f32>(0.));
  let m2 = m * m;
  let m4 = m2 * m2;
  let pdotx = vec4(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3));

  // Determine noise gradient
  let temp = m2 * m * pdotx;
  var grad = -8.0 * (temp.x * x0 + temp.y * x1 + temp.z * x2 + temp.w * x3);
  grad += m4.x * p0 + m4.y * p1 + m4.z * p2 + m4.w * p3;
  grad *= 105.0;

  *gradient = grad;
  return 105.0 * dot(m4, pdotx);
}
