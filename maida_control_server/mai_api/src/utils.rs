
pub fn single_ra(achievements: i32, constant: f32) -> i32 {
    let achievements=achievements as f32 / 10000.0;
    let ach = achievements.min(100.5);
    let mul = get_coefficient(ach);
    ((constant * ach * mul).floor() as i32)
}
pub fn get_coefficient(achievements: f32) -> f32 {
    // 由于 DA 达成率最大 capped 为 100.5%
    let a = achievements.min(100.5);

    match a {
        x if x >= 100.5   => 0.224,  // SSS+
        x if x >= 100.4999=> 0.222,  // SSS
        x if x >= 100.0   => 0.216,  // SSS
        x if x >= 99.9999 => 0.214,  // SS+
        x if x >= 99.5    => 0.211,  // SS+
        x if x >= 99.0    => 0.208,  // SS
        x if x >= 98.9999 => 0.206,  // S+
        x if x >= 98.0    => 0.203,  // S+
        x if x >= 97.0    => 0.200,  // S
        x if x >= 96.9999 => 0.176,  // AAA
        x if x >= 94.0    => 0.168,  // AAA
        x if x >= 90.0    => 0.152,  // AA
        x if x >= 80.0    => 0.136,  // A
        x if x >= 79.9999 => 0.128,  // BBB
        x if x >= 75.0    => 0.120,  // BBB
        x if x >= 70.0    => 0.112,  // BB
        x if x >= 60.0    => 0.096,  // B
        x if x >= 50.0    => 0.080,  // C
        x if x >= 40.0    => 0.064,  // D
        x if x >= 30.0    => 0.048,
        x if x >= 20.0    => 0.032,
        x if x >= 10.0    => 0.016,
        _                 => 0.0,
    }
}


#[test]
fn test(){
    let ra = single_ra(1003272, 13.7);
    print!("{ra}")
}