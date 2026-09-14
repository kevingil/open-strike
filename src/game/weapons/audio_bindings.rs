//! Explicit bindings to local recording IDs, with bundled cues as fallbacks.
use crate::game::config::WeaponId;

pub(super) struct WeaponSoundIds {
    pub shot: &'static str,
    pub draw: &'static str,
    pub reload: &'static [(f32, &'static str)],
}

/// Reload fractions follow the simulation duration. Only AK has authored
/// first/third-person magazine timings; other guns use staged handling cues.
pub(super) fn sound_ids(weapon: WeaponId) -> WeaponSoundIds {
    use WeaponId::*;
    match weapon {
        Glock18 => WeaponSoundIds {
            shot: "weapons/glock18-1",
            draw: "weapons/glock_draw",
            reload: &[
                (18.0 / 99.0, "weapons/glock_clipout"),
                (65.0 / 99.0, "weapons/glock_clipin"),
                (73.0 / 99.0, "weapons/glock_sliderelease"),
            ],
        },
        Usps => WeaponSoundIds {
            shot: "weapons/usp1",
            draw: "weapons/usp_slideback",
            reload: &[
                (18.0 / 99.0, "weapons/usp_clipout"),
                (65.0 / 99.0, "weapons/usp_clipin"),
                (73.0 / 99.0, "weapons/usp_sliderelease"),
            ],
        },
        P2000 => WeaponSoundIds {
            shot: "weapons/hkp2000-1",
            draw: "weapons/hkp2000_slideback",
            reload: &[
                (18.0 / 99.0, "weapons/hkp2000_clipout"),
                (65.0 / 99.0, "weapons/hkp2000_clipin"),
                (73.0 / 99.0, "weapons/hkp2000_sliderelease"),
            ],
        },
        P250 => WeaponSoundIds {
            shot: "weapons/p250-1",
            draw: "weapons/p250_slideback",
            reload: &[
                (18.0 / 99.0, "weapons/p250_clipout"),
                (65.0 / 99.0, "weapons/p250_clipin"),
                (73.0 / 99.0, "weapons/p250_sliderelease"),
            ],
        },
        Tec9 => WeaponSoundIds {
            shot: "weapons/tec9-1",
            draw: "weapons/tec9_draw",
            reload: &[
                (18.0 / 99.0, "weapons/tec9_clipout"),
                (65.0 / 99.0, "weapons/tec9_clipin"),
                (73.0 / 99.0, "weapons/tec9_boltrelease"),
            ],
        },
        FiveSeven => WeaponSoundIds {
            shot: "weapons/fiveseven-1",
            draw: "weapons/fiveseven_slideback",
            reload: &[
                (18.0 / 99.0, "weapons/fiveseven_clipout"),
                (65.0 / 99.0, "weapons/fiveseven_clipin"),
                (73.0 / 99.0, "weapons/fiveseven_sliderelease"),
            ],
        },
        Cz75 => WeaponSoundIds {
            shot: "weapons/cz75a-1",
            draw: "weapons/p250_slideback",
            reload: &[
                (18.0 / 99.0, "weapons/p250_clipout"),
                (65.0 / 99.0, "weapons/p250_clipin"),
                (73.0 / 99.0, "weapons/p250_sliderelease"),
            ],
        },
        Deagle => WeaponSoundIds {
            shot: "weapons/deagle-1",
            draw: "weapons/de_slideback",
            reload: &[
                (18.0 / 99.0, "weapons/de_clipout"),
                (65.0 / 99.0, "weapons/de_clipin"),
                (73.0 / 99.0, "weapons/de_slideforward"),
            ],
        },
        R8 => WeaponSoundIds {
            shot: "weapons/revolver-1_01",
            draw: "weapons/revolver_draw",
            reload: &[
                (18.0 / 99.0, "weapons/revolver_clipout"),
                (65.0 / 99.0, "weapons/revolver_clipin"),
                (73.0 / 99.0, "weapons/revolver_siderelease"),
            ],
        },
        Mac10 => WeaponSoundIds {
            shot: "weapons/mac10-1",
            draw: "weapons/mac10_draw",
            reload: &[
                (18.0 / 99.0, "weapons/mac10_clipout"),
                (65.0 / 99.0, "weapons/mac10_clipin"),
                (73.0 / 99.0, "weapons/mac10_boltforward"),
            ],
        },
        Mp9 => WeaponSoundIds {
            shot: "weapons/mp9-1",
            draw: "weapons/mp9_draw",
            reload: &[
                (18.0 / 99.0, "weapons/mp9_clipout"),
                (65.0 / 99.0, "weapons/mp9_clipin"),
                (73.0 / 99.0, "weapons/mp9_boltforward"),
            ],
        },
        Mp7 => WeaponSoundIds {
            shot: "weapons/mp7-1",
            draw: "weapons/mp7_draw",
            reload: &[
                (18.0 / 99.0, "weapons/mp7_clipout"),
                (65.0 / 99.0, "weapons/mp7_clipin"),
                (73.0 / 99.0, "weapons/mp7_slideforward"),
            ],
        },
        Mp5sd => WeaponSoundIds {
            shot: "weapons/mp5_01",
            draw: "weapons/mp5_draw",
            reload: &[
                (18.0 / 99.0, "weapons/mp5_clipout"),
                (65.0 / 99.0, "weapons/mp5_clipin"),
                (73.0 / 99.0, "weapons/mp5_slideforward"),
            ],
        },
        Ump45 => WeaponSoundIds {
            shot: "weapons/ump45-1",
            draw: "weapons/ump45_draw",
            reload: &[
                (18.0 / 99.0, "weapons/ump45_clipout"),
                (65.0 / 99.0, "weapons/ump45_clipin"),
                (73.0 / 99.0, "weapons/ump45_boltforward"),
            ],
        },
        P90 => WeaponSoundIds {
            shot: "weapons/p90-1",
            draw: "weapons/p90_draw",
            reload: &[
                (18.0 / 99.0, "weapons/p90_clipout"),
                (65.0 / 99.0, "weapons/p90_clipin"),
                (73.0 / 99.0, "weapons/p90_boltforward"),
            ],
        },
        Bizon => WeaponSoundIds {
            shot: "weapons/bizon-1",
            draw: "weapons/bizon_draw",
            reload: &[
                (18.0 / 99.0, "weapons/bizon_clipout"),
                (65.0 / 99.0, "weapons/bizon_clipin"),
                (73.0 / 99.0, "weapons/bizon_boltforward"),
            ],
        },
        Galil => WeaponSoundIds {
            shot: "weapons/galil-1",
            draw: "weapons/galil_draw",
            reload: &[
                (18.0 / 99.0, "weapons/galil_clipout"),
                (65.0 / 99.0, "weapons/galil_clipin"),
                (73.0 / 99.0, "weapons/galil_boltforward"),
            ],
        },
        Famas => WeaponSoundIds {
            shot: "weapons/famas-1",
            draw: "weapons/famas_draw",
            reload: &[
                (18.0 / 99.0, "weapons/famas_clipout"),
                (65.0 / 99.0, "weapons/famas_clipin"),
                (73.0 / 99.0, "weapons/famas_boltforward"),
            ],
        },
        AK47 => WeaponSoundIds {
            shot: "weapons/ak47-1",
            draw: "weapons/ak47_draw",
            reload: &[
                (18.0 / 99.0, "weapons/ak47_clipout"),
                (65.0 / 99.0, "weapons/ak47_clipin"),
                (73.0 / 99.0, "weapons/ak47_boltpull"),
            ],
        },
        M4a4 => WeaponSoundIds {
            shot: "weapons/m4a1_unsil-1",
            draw: "weapons/m4a1_draw",
            reload: &[
                (18.0 / 99.0, "weapons/m4a1_clipout"),
                (65.0 / 99.0, "weapons/m4a1_clipin"),
                (73.0 / 99.0, "weapons/m4a1_boltforward"),
            ],
        },
        M4a1s => WeaponSoundIds {
            shot: "weapons/m4a1_silencer_01",
            draw: "weapons/m4a1_draw",
            reload: &[
                (18.0 / 99.0, "weapons/m4a1_clipout"),
                (65.0 / 99.0, "weapons/m4a1_clipin"),
                (73.0 / 99.0, "weapons/m4a1_silencer_boltforward"),
            ],
        },
        Sg553 => WeaponSoundIds {
            shot: "weapons/sg556-1",
            draw: "weapons/sg556_draw",
            reload: &[
                (18.0 / 99.0, "weapons/sg556_clipout"),
                (65.0 / 99.0, "weapons/sg556_clipin"),
                (73.0 / 99.0, "weapons/sg556_boltforward"),
            ],
        },
        Aug => WeaponSoundIds {
            shot: "weapons/aug-1",
            draw: "weapons/aug_boltpull",
            reload: &[
                (18.0 / 99.0, "weapons/aug_clipout"),
                (65.0 / 99.0, "weapons/aug_clipin"),
                (73.0 / 99.0, "weapons/aug_boltrelease"),
            ],
        },
        Ssg08 => WeaponSoundIds {
            shot: "weapons/ssg08-1",
            draw: "weapons/ssg08_draw",
            reload: &[
                (18.0 / 99.0, "weapons/ssg08_clipout"),
                (65.0 / 99.0, "weapons/ssg08_clipin"),
                (73.0 / 99.0, "weapons/ssg08_boltforward"),
            ],
        },
        Awp => WeaponSoundIds {
            shot: "weapons/awp1",
            draw: "weapons/awp_boltback",
            reload: &[
                (18.0 / 99.0, "weapons/awp_clipout"),
                (65.0 / 99.0, "weapons/awp_clipin"),
                (73.0 / 99.0, "weapons/awp_boltforward"),
            ],
        },
        G3sg1 => WeaponSoundIds {
            shot: "weapons/g3sg1-1",
            draw: "weapons/g3sg1_draw",
            reload: &[
                (18.0 / 99.0, "weapons/g3sg1_clipout"),
                (65.0 / 99.0, "weapons/g3sg1_clipin"),
                (73.0 / 99.0, "weapons/g3sg1_slideforward"),
            ],
        },
        Scar20 => WeaponSoundIds {
            shot: "weapons/scar20_unsil-1",
            draw: "weapons/scar20_draw",
            reload: &[
                (18.0 / 99.0, "weapons/scar20_clipout"),
                (65.0 / 99.0, "weapons/scar20_clipin"),
                (73.0 / 99.0, "weapons/scar20_boltforward"),
            ],
        },
        DualBerettas => WeaponSoundIds {
            shot: "weapons/elite-1",
            draw: "weapons/elite_draw",
            reload: &[
                (0.15, "weapons/elite_clipout"),
                (0.4, "weapons/elite_leftclipin"),
                (0.65, "weapons/elite_rightclipin"),
                (0.85, "weapons/elite_sliderelease"),
            ],
        },
        Nova => WeaponSoundIds {
            shot: "weapons/nova-1",
            draw: "weapons/nova_draw",
            reload: &[
                (0.2, "weapons/nova_insertshell"),
                (0.4, "weapons/nova_insertshell"),
                (0.6, "weapons/nova_insertshell"),
                (0.85, "weapons/nova_pump"),
            ],
        },
        Xm1014 => WeaponSoundIds {
            shot: "weapons/xm1014-1",
            draw: "weapons/xm1014_pump",
            reload: &[
                (0.2, "weapons/xm1014_insertshell"),
                (0.4, "weapons/xm1014_insertshell"),
                (0.6, "weapons/xm1014_insertshell"),
                (0.85, "weapons/xm1014_pump"),
            ],
        },
        SawedOff => WeaponSoundIds {
            shot: "weapons/sawedoff-1",
            draw: "weapons/sawedoff_draw",
            reload: &[
                (0.2, "weapons/sawedoff_insertshell_01"),
                (0.4, "weapons/sawedoff_insertshell_02"),
                (0.6, "weapons/sawedoff_insertshell_03"),
                (0.85, "weapons/sawedoff_pump"),
            ],
        },
        M249 => WeaponSoundIds {
            shot: "weapons/m249-1",
            draw: "weapons/m249_draw",
            reload: &[
                (0.1, "weapons/m249_coverup"),
                (0.25, "weapons/m249_boxout"),
                (0.55, "weapons/m249_boxin"),
                (0.7, "weapons/m249_chain"),
                (0.9, "weapons/m249_coverdown"),
            ],
        },
        Negev => WeaponSoundIds {
            shot: "weapons/negev-1",
            draw: "weapons/movement1",
            reload: &[
                (0.1, "weapons/negev_coverup"),
                (0.25, "weapons/negev_boxout"),
                (0.55, "weapons/negev_boxin"),
                (0.7, "weapons/negev_chain"),
                (0.9, "weapons/negev_coverdown"),
            ],
        },
        DefaultKnife | DefaultKnifeStained | DefaultKnifeForest | DefaultKnifeCt
        | DefaultKnifeT | ReferenceKnife | Karambit => WeaponSoundIds {
            shot: "weapons/knife_slash",
            draw: "weapons/knife_draw",
            reload: &[],
        },
    }
}
