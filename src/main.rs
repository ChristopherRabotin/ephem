extern crate nalgebra;
//extern crate nyx;
use nalgebra::{Point3, Vector3, Vector6};
use nalgebra::geometry::{Isometry3, IsometryMatrix3, Quaternion, UnitQuaternion};
use nalgebra::Real;

fn main() {
    /*let eph = nyx::ephemeris::bsp::from_path("de436.bsp");
    // Don't lose track of the goal here. It isn't to get the positions and velocities of the planets
    // but specifically to transform from one frame to another.
    let earth_now = eph.position(nyx::bodies::Earth, some_time);
    eph.quaternion*/
    let o = Point3::new(0.0, 0.0, 0.0);
    let op = Point3::new(1.0, 1.0, 0.0);
    let opp = Point3::new(2.0, 2.0, 0.0);
    let p = Point3::new(2.0, 1.0, 0.0);
    let p_op = Point3::new(1.0, 0.0, 0.0);
    let p_opp = Point3::new(0.0, -1.0, 0.0);
    let op_vec = o - op;
    let opp_vec = o - opp;
    let iso_o2op = Isometry3::new(op_vec, nalgebra::zero());
    let iso_o2opp = Isometry3::new(opp_vec, nalgebra::zero());
    let iso_direct = Isometry3::new(op - opp, nalgebra::zero());
    println!("op_vec: {:}", iso_o2op * p);
    println!("opp_vec: {:}", iso_o2opp * p);
    println!("direct: {:}", iso_direct * (iso_o2op * p));
}
