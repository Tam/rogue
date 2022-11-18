use rltk::XpFile;

rltk::embedded_resource!(DUNGEON_BG, "../resources/dungeon-bg.xp");
rltk::embedded_resource!(WFC_DEMO_IMAGE1, "../resources/wfc-demo1.xp");

pub struct RexAssets {
	pub menu : XpFile,
}

impl RexAssets {
	pub fn new() -> RexAssets {
		rltk::link_resource!(DUNGEON_BG, "../resources/dungeon-bg.xp");
		rltk::link_resource!(WFC_DEMO_IMAGE1, "../resources/wfc-demo1.xp");

		RexAssets {
			menu: XpFile::from_resource("../resources/dungeon-bg.xp").unwrap(),
		}
	}
}