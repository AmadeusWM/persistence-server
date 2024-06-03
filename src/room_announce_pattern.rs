#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomAnnouncePattern {
    pub index_namespace: String,
    pub prefix: String,
    pub room_id: String,
    pub publisher_id: String,
}

impl RoomAnnouncePattern {
    pub fn new(
        index_namespace: String,
        prefix: String,
        room_id: String,
        publisher_id: String,
    ) -> Self {
        Self {
            index_namespace,
            prefix,
            room_id,
            publisher_id,
        }
    }

    /// Parse an announce string into its parts
    ///
    /// # Example
    /// ```
    /// parse_announce(
    ///    ".".to_string(),
    ///    "room.participant.".to_string(),
    ///    ".room.participant.X.A".to_string()
    /// ); // room_id = "X", publisher_id = "A"
    /// ```
    pub fn parse_announce(
        index_namespace: String,
        prefix: String,
        announce: String,
    ) -> Option<Self> {
        let pattern = format!("{}{}", index_namespace, prefix);
        if announce.starts_with(pattern.as_str()) {
            let (_lead, trail) = announce.split_at(pattern.len());
            let split: Vec<&str> = trail.split(".").collect();
            if let Some([room_id, publisher_id]) = split.get(0..2) {
                return Some(Self {
                    index_namespace,
                    prefix,
                    room_id: room_id.to_string(),
                    publisher_id: publisher_id.to_string(),
                });
            }
        }
        None
    }

    pub fn to_namespace(&self) -> String {
        return format!("{}{}{}.{}", self.index_namespace, self.prefix, self.room_id, self.publisher_id)
    }

    pub fn to_index_track(&self) -> String {
        return format!("{}{}.", self.prefix, self.room_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_room_announce_pattern() {
        let index_namespace = ".".to_string();
        let prefix_participant = "room.participant.".to_string();
        let prefix_room = "room.provider.".to_string();
        assert_eq!(
            RoomAnnouncePattern::parse_announce(
                index_namespace.clone(),
                prefix_participant.clone(),
                ".room.participant.xyz.abc".to_string()
            ).unwrap(),
            RoomAnnouncePattern::new(
                index_namespace.clone(),
                prefix_participant.clone(),
                "xyz".to_string(),
                "abc".to_string()
            )
        );

        assert_eq!(
            RoomAnnouncePattern::parse_announce(
                index_namespace.clone(),
                prefix_room.clone(),
                ".room.provider.xyz.abc".to_string()
            ).unwrap(),
            RoomAnnouncePattern::new(
                index_namespace.clone(),
                prefix_room.clone(),
                "xyz".to_string(),
                "abc".to_string()
            )
        );

        assert_eq!(
            RoomAnnouncePattern::parse_announce(
                index_namespace.clone(),
                prefix_participant.clone(),
                ".room.participantxyz.abc".to_string()
            ),
            None,
        );
    }
}
