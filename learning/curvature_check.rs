//! 微分几何入门练习：用数值方法验证曲率公式，并检查"滚动球圆角"在接缝处是否 G1 / G2。
//!
//! 只用标准库，不依赖仓库其它 crate：
//!
//! ```sh
//! rustc -O learning/curvature_check.rs -o /tmp/curvature_check
//! /tmp/curvature_check
//! ```
//!
//! 对应 Pressley《Elementary Differential Geometry》：
//! - 第 6 章：第一基本形式 E, F, G
//! - 第 7 章：第二基本形式 L, M, N
//! - 第 8 章：高斯曲率 K、平均曲率 H、主曲率 κ₁/κ₂
//!
//! 曲面写成 S(u, v)；E/F/G 和 L/M/N 用中心差分算导数，再套教科书公式，
//! 和解析值对比。这样你写的每个公式都能当场验证。

#[derive(Clone, Copy, Debug)]
struct V3 {
    x: f64,
    y: f64,
    z: f64,
}

impl V3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    fn mul(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
    fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn cross(self, o: Self) -> Self {
        Self::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
    fn unit(self) -> Self {
        self.mul(1.0 / self.norm())
    }
}

type Surf<'a> = &'a dyn Fn(f64, f64) -> V3;

/// 第一基本形式 (E, F, G) 与第二基本形式 (L, M, N)。
#[derive(Clone, Copy)]
struct Forms {
    e: f64,
    f: f64,
    g: f64,
    l: f64,
    m: f64,
    n: f64,
}

fn forms(s: Surf, u: f64, v: f64) -> Forms {
    let h = 1e-5;
    let su = s(u + h, v).sub(s(u - h, v)).mul(0.5 / h);
    let sv = s(u, v + h).sub(s(u, v - h)).mul(0.5 / h);
    let suu = s(u + h, v).add(s(u - h, v)).sub(s(u, v).mul(2.0)).mul(1.0 / (h * h));
    let svv = s(u, v + h).add(s(u, v - h)).sub(s(u, v).mul(2.0)).mul(1.0 / (h * h));
    let suv = s(u + h, v + h).sub(s(u + h, v - h)).sub(s(u - h, v + h)).add(s(u - h, v - h)).mul(1.0 / (4.0 * h * h));
    let normal = su.cross(sv).unit();
    // 符号约定：II = -S_uu·N（等价于 II = -dN·dS），让凸曲面（球、圆柱外侧）
    // 的平均曲率为正。K = (LN-M²)/(EG-F²) 与符号无关；H 会整体变号，
    // 和某些教材差一个整体负号，比较时注意。
    Forms {
        e: su.dot(su),
        f: su.dot(sv),
        g: sv.dot(sv),
        l: -suu.dot(normal),
        m: -suv.dot(normal),
        n: -svv.dot(normal),
    }
}

fn normal(s: Surf, u: f64, v: f64) -> V3 {
    let h = 1e-5;
    let su = s(u + h, v).sub(s(u - h, v));
    let sv = s(u, v + h).sub(s(u, v - h));
    su.cross(sv).unit()
}

impl Forms {
    /// 高斯曲率 K = (LN - M²) / (EG - F²)。
    fn gaussian(&self) -> f64 {
        (self.l * self.n - self.m * self.m) / (self.e * self.g - self.f * self.f)
    }

    /// 平均曲率 H = (EN - 2FM + GL) / 2(EG - F²)。
    fn mean(&self) -> f64 {
        (self.e * self.n - 2.0 * self.f * self.m + self.g * self.l) / (2.0 * (self.e * self.g - self.f * self.f))
    }

    /// 主曲率 κ₁ ≤ κ₂ = H ∓ √(H² - K)。
    fn principal(&self) -> (f64, f64) {
        let h = self.mean();
        let k = self.gaussian();
        let d = (h * h - k).max(0.0).sqrt();
        (h - d, h + d)
    }
}

// ---------- CAD 内核里最常见的几张曲面 ----------

/// 半径 r 的球：u = 经度，v = 纬度。
fn sphere(r: f64) -> impl Fn(f64, f64) -> V3 {
    move |u, v| V3::new(r * v.cos() * u.cos(), r * v.cos() * u.sin(), r * v.sin())
}

/// 半径 r、高 h 的圆柱：u = 绕轴角，v = 高度。
fn cylinder(r: f64, h: f64) -> impl Fn(f64, f64) -> V3 {
    move |u, v| V3::new(r * u.cos(), r * u.sin(), v.clamp(0.0, h))
}

/// 环面：大半径 big、管半径 tube；u 绕主轴，v 绕管（0 = 外侧赤道）。
fn torus(big: f64, tube: f64) -> impl Fn(f64, f64) -> V3 {
    move |u, v| {
        let rad = big + tube * v.cos();
        V3::new(rad * u.cos(), rad * u.sin(), tube * v.sin())
    }
}

// ---------- 检查 ----------

struct Case {
    name: &'static str,
    got: f64,
    want: f64,
    tol: f64,
}

fn main() {
    let mut cases: Vec<Case> = Vec::new();
    let plane = |u: f64, v: f64| V3::new(u, v, 0.0);

    // 1) 平面：K = 0, H = 0
    let f = forms(&plane, 3.0, -2.0);
    cases.push(Case { name: "plane K", got: f.gaussian(), want: 0.0, tol: 1e-6 });
    cases.push(Case { name: "plane H", got: f.mean(), want: 0.0, tol: 1e-6 });

    // 2) 球：K = 1/r², H = 1/r
    let r = 50.0;
    let s = sphere(r);
    let f = forms(&s, 0.7, -0.4);
    cases.push(Case { name: "sphere K", got: f.gaussian(), want: 1.0 / (r * r), tol: 1e-6 });
    cases.push(Case { name: "sphere H", got: f.mean(), want: 1.0 / r, tol: 1e-6 });

    // 3) 圆柱：K = 0, H = 1/(2r)
    let c = cylinder(r, 41.0);
    let f = forms(&c, 1.1, 20.0);
    cases.push(Case { name: "cylinder K", got: f.gaussian(), want: 0.0, tol: 1e-6 });
    cases.push(Case { name: "cylinder H", got: f.mean(), want: 1.0 / (2.0 * r), tol: 1e-6 });

    // 4) 环面：K = cos v / (tube (big + tube cos v))
    //          H = (big + 2 tube cos v) / (2 tube (big + tube cos v))
    let (big, tube) = (40.0, 10.0);
    let t = torus(big, tube);
    let v = 0.9;
    let f = forms(&t, 2.2, v);
    let den = tube * (big + tube * v.cos());
    cases.push(Case { name: "torus K", got: f.gaussian(), want: v.cos() / den, tol: 1e-6 });
    cases.push(Case { name: "torus H", got: f.mean(), want: (big + 2.0 * tube * v.cos()) / (2.0 * den), tol: 1e-6 });

    println!("== 1. 曲率公式：数值 vs 解析 ==");
    let mut failed = 0;
    for c in &cases {
        let ok = (c.got - c.want).abs() < c.tol;
        if !ok {
            failed += 1;
        }
        println!("{:<12} numeric = {:+.10}   analytic = {:+.10}   {}", c.name, c.got, c.want, if ok { "ok" } else { "MISMATCH" });
    }

    // 5) 滚动球圆角：圆柱 R + 底部倒角 f
    //
    //    圆角面 = 环面：大半径 R-f、管半径 f，圆心在 z = f。
    //      v = 0    → 与圆柱侧面相接（半径 R，z = f）
    //      v = -π/2 → 与底面 z = 0 相接（半径 R-f）
    //
    //    检查接缝处：法向是否一致（G1）、主曲率是否一致（G2）。
    println!("\n== 2. 滚动球圆角：G1 还是 G2？ ==");
    let (big_r, fillet) = (50.0, 10.0);
    let cyl = cylinder(big_r, 41.0);
    let fil = torus(big_r - fillet, fillet);
    let fillet_surface = |u: f64, v: f64| {
        let p = fil(u, v);
        V3::new(p.x, p.y, p.z + fillet)
    };

    let u = 0.8;
    for (label, v, against_cylinder) in
        [("接圆柱侧面", 0.0, true), ("接底面", -std::f64::consts::FRAC_PI_2, false)]
    {
        let p = fillet_surface(u, v);
        let n_fillet = normal(&fillet_surface, u, v);
        let (n_other, k_other) = if against_cylinder {
            (normal(&cyl, u, 20.0), forms(&cyl, u, 20.0).principal())
        } else {
            (V3::new(0.0, 0.0, -1.0), (0.0, 0.0))
        };
        let k_fillet = forms(&fillet_surface, u, v).principal();
        let cos = n_fillet.dot(n_other);
        let g1 = (cos - 1.0).abs() < 1e-6;
        let g2 = (k_fillet.0 - k_other.0).abs() < 1e-6 && (k_fillet.1 - k_other.1).abs() < 1e-6;
        println!("\n{label}: 接缝点 z = {:.1}", p.z);
        println!("  法向点积 = {cos:+.9}   → {}", if g1 { "G1（相切）" } else { "不连续" });
        println!(
            "  圆角主曲率 = ({:+.6}, {:+.6})   另一侧 = ({:+.6}, {:+.6})",
            k_fillet.0, k_fillet.1, k_other.0, k_other.1
        );
        println!("  → {}", if g2 { "G2（曲率也一致）" } else { "G1 但非 G2（有一个主曲率跳变）" });
    }

    // 6) 曲率决定网格密度：弦高 = R(1 - cos(θ/2)) ≈ R θ²/8
    println!("\n== 3. 曲率决定网格密度（弦高容差） ==");
    for &(radius, tol) in &[(50.0f64, 0.1f64), (50.0, 0.01), (5.0, 0.01)] {
        let theta = (8.0 * tol / radius).sqrt();
        let segments = (std::f64::consts::TAU / theta).ceil();
        println!("R = {radius:5.1} mm，允许弦高 {tol:4.2} mm → 每圈至少 {segments:4.0} 段（θ ≈ {theta:.4} rad）");
    }

    if failed > 0 {
        eprintln!("\n{failed} 项不匹配：检查公式或差分步长 h。");
        std::process::exit(1);
    }
    println!("\n全部曲率检查通过。");
}
