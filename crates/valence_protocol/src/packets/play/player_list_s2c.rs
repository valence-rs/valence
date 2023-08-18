use super::*;

#[derive(Clone, Debug, Packet)]
pub struct PlayerListS2c<'a> {
    pub actions: PlayerListActions,
    pub entries: Cow<'a, [PlayerListEntry<'a>]>,
}

impl<'a> Encode for PlayerListS2c<'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.actions.0.encode(&mut w)?;

        // Write number of entries.
        VarInt(self.entries.len() as i32).encode(&mut w)?;

        for entry in self.entries.as_ref() {
            entry.player_uuid.encode(&mut w)?;

            if self.actions.add_player() {
                entry.username.encode(&mut w)?;
                entry.properties.encode(&mut w)?;
            }

            if self.actions.initialize_chat() {
                entry.chat_data.encode(&mut w)?;
            }

            if self.actions.update_game_mode() {
                entry.game_mode.encode(&mut w)?;
            }

            if self.actions.update_listed() {
                entry.listed.encode(&mut w)?;
            }

            if self.actions.update_latency() {
                VarInt(entry.ping).encode(&mut w)?;
            }

            if self.actions.update_display_name() {
                entry.display_name.encode(&mut w)?;
            }
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for PlayerListS2c<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let actions = PlayerListActions(u8::decode(r)?);

        let mut entries = vec![];

        for _ in 0..VarInt::decode(r)?.0 {
            let mut entry = PlayerListEntry {
                player_uuid: Uuid::decode(r)?,
                ..Default::default()
            };

            if actions.add_player() {
                entry.username = Decode::decode(r)?;
                entry.properties = Decode::decode(r)?;
            }

            if actions.initialize_chat() {
                entry.chat_data = Decode::decode(r)?;
            }

            if actions.update_game_mode() {
                entry.game_mode = Decode::decode(r)?;
            }

            if actions.update_listed() {
                entry.listed = Decode::decode(r)?;
            }

            if actions.update_latency() {
                entry.ping = VarInt::decode(r)?.0;
            }

            if actions.update_display_name() {
                entry.display_name = Decode::decode(r)?;
            }

            entries.push(entry);
        }

        Ok(Self {
            actions,
            entries: entries.into(),
        })
    }
}

#[bitfield(u8)]
pub struct PlayerListActions {
    pub add_player: bool,
    pub initialize_chat: bool,
    pub update_game_mode: bool,
    pub update_listed: bool,
    pub update_latency: bool,
    pub update_display_name: bool,
    #[bits(2)]
    _pad: u8,
}

#[derive(Clone, Default, Debug)]
pub struct PlayerListEntry<'a> {
    pub player_uuid: Uuid,
    pub username: &'a str,
    pub properties: Cow<'a, [PropertyValue]>,
    pub chat_data: Option<ChatData<'a>>,
    pub listed: bool,
    pub ping: i32,
    pub game_mode: GameMode,
    pub display_name: Option<Cow<'a, Text>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct ChatData<'a> {
    pub session_id: Uuid,
    /// Unix timestamp in milliseconds.
    pub key_expiry_time: i64,
    pub public_key: &'a [u8],
    pub public_key_signature: &'a [u8],
}
