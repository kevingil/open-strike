"""Shared Sketchfab catalog for Open Strike weapons.

Used by download_catalog.py and export_catalog.py. Keys match WeaponId::key().
"""

from pathlib import Path
from typing import Optional

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/models/catalog"
GENERATED = ROOT / "assets/generated"

# uid, author, title, page, notes
WEAPONS = [
    # pistols
    ("glock18", "fdd5cdd7b31c49f2b16a868ebdf4a75f", "IVA", "G18", "silencer=keep"),
    ("usps", "972e3643000246e8959a6df509557bcc", "Urpo", "Heckler & Koch USP Pistol", "silencer=add"),
    ("p2000", "867b41f7121d4b9b88b89cc73017f2d4", "AvnisT", "P2000", ""),
    ("dual_berettas", "474cb73931114a669780783c44b5bb88", "eNse7en", "Beretta 92G Brigadier Elite II", "silencer=strip dual=1"),
    ("p250", "75f910f1346247179627c2ae54aa9978", "Ulrik Langvandsbråten", "Sig Sauer P250 (Anim)", ""),
    ("tec9", "fed8b9bad25242b8ba7c81d675ff050f", "wallon", "Tec-9", ""),
    ("fiveseven", "4583bf36ff2644e2800b6719fac274dc", "simon_fischer", "FN Five-seveN", ""),
    ("cz75", "4b62fc0811a7436e8caeba730958f9e0", "tomnortheast99", "CZ-75 Auto", ""),
    ("deagle", "cabde59f5cf24effaf80536e35d04e95", "ELIZION", "Desert Eagle", ""),
    ("r8", "69f38c63e62c44a180bda09a17820ea5", "Oxenci", "Taurus - Raging Hunter 460", ""),
    # mid-tier
    ("mac10", "f185b6c2a4b24b0b9b530c47fac73be7", "Vasily", "Ingram MAC-10", "silencer=strip"),
    ("mp9", "18ea203c57b5477c8dcb8ef2768423b2", "GoldbergR", "B&T MP9 (Low-Poly)", ""),
    ("mp7", "27cd4b48fb14451c86adad4c559082d6", "Steve Henry", "H&K MP7 A1", "silencer=strip"),
    ("mp5sd", "2b96ccb2dc51496e93f41734663bcadf", "WillyG99", "MP5SD (Silenced MP5)", ""),
    ("ump45", "53ec6320f1c84960a5d49fa0b7a11480", "Levi Giovani", "UMP-45 (FREE)", ""),
    ("p90", "19b39a4ee12442078c7c79c52118e01f", "commiessar", "P90 Sub Machinegun", ""),
    ("bizon", "b6c6364131f04be9b5691f323e6d391f", "AvnisT", "PP-19 Bizon", ""),
    ("nova", "8c091501adeb493a997544fee3da50f6", "Beerus", "Shotgun - NOVA", ""),
    ("xm1014", "e0d23db5a8b64ff2980875834b425c4b", "drollShark", "Shotgun Benelli M4", ""),
    ("sawedoff", "806f8327bed34fc3b0e3911d01f0566d", "DJMaesen", "sawnoff animated", ""),
    ("m249", "76011c365636451c90a8e3a46c2d8ca5", "TastyTony", "Low-Poly M249 SAW", ""),
    ("negev", "68ff380279c54f1d9ab255c75fe503d2", "GoldbergR", "IWI Negev NG-5", ""),
    # rifles
    ("galil", "b36a4fec55b148d08afa3f135cd9a807", "ScurvyWoof", "Galil ACE 23 (low-poly)", ""),
    ("famas", "7d35e14ad8874d148ce9cc46e5765a6b", "Frostoise", "FAMAS", ""),
    ("m4a4", "222fd3948aab45eb9d0cbced9c80308a", "blazitt", "M4A4 Counter Strike 2", ""),
    ("m4a1s", "de142447b4d047bbb0311c2974520f1e", "puresaltt", "Low Poly Colt M4A1", "silencer=add"),
    ("sg553", "2e9ebf6ecc004e8a82f5fb5117f0eb10", "D_U", "low-poly SIG SG-553", "scope=add"),
    ("aug", "73e9323ba9ed40bfb308babd29c68280", "Al", "Aug A3 M1with Aimpoint micro T2", "scope=strip"),
    ("ssg08", "3f3daf78b04149aa867abfba231fadf8", "Rifeor", "ssg 08| Fever Dream", "paint=black scope=keep"),
    ("awp", "b7101f0325aa4b0dad8512d0ec67bfa1", "forestie", "AWP", "scope=keep"),
    ("g3sg1", "21b8e3bceca64b00856c1e9abcca8573", "Javier Clemente García", "HK mod. G3SG/1", "scope=keep"),
    ("scar20", "5951c446c76343e2a03bee582881e417", "wertat", "FN-SCAR L", "scope=keep"),
    # knives
    ("knife_ct", "3ae258cee29b44358efec2c7bbe7b59d", "D_U", "low-poly KM2000", "knife=1"),
    ("knife_t", "5fb333837f1343caa6a0e80a46b68ac1", "lunea", "Tactical Knife", "knife=1"),
    ("karambit", "dfd7606f189a413681305a39b8841ce8", "Diamonddogkz", "Karambit", "knife=1"),
]

SILENCER_DONOR = ("dual_berettas", "474cb73931114a669780783c44b5bb88")

# Existing in-repo sources; not downloaded.
LOCAL = {
    "ak47": ROOT / "assets/models/weapons/ak_47.glb",
    "knife": None,
    "knife_stained": None,
    "knife_forest": None,
    "reference_knife": ROOT / "assets/models/viewmodels/knife_animated.glb",
}


def source_path(key: str) -> Optional[Path]:
    if key in LOCAL and LOCAL[key] is not None:
        return LOCAL[key]
    folder = SOURCE / key
    if not folder.exists():
        return None
    for pattern in ("*.glb", "*.gltf", "*.fbx", "*.obj", "*.blend"):
        hits = sorted(folder.rglob(pattern))
        if hits:
            return hits[0]
    return None
