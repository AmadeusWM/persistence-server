#[derive(Debug, PartialEq, Eq)]
pub enum IndexPacket {
    Insert(String),
    Remove(String),
    Snapshot(Vec<String>),
}

impl From<String> for IndexPacket {
    fn from(value: String) -> Self {
        if value.len() == 0 {
            return IndexPacket::Snapshot(vec![]);
        }
        if value.contains("\n") {
            let parts: Vec<String> = value
                .split("\n")
                .map(String::from)
                .filter(|s| !s.is_empty())
                .collect();
            return IndexPacket::Snapshot(parts);
        }

        match value.split_at(1) {
            ("+", value) => Self::Insert(value.to_string()),
            ("-", value) => Self::Remove(value.to_string()),
            _ => Self::Snapshot(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_index_packet_parse() {
        assert_eq!(
            IndexPacket::from(String::from("+a")),
            IndexPacket::Insert(String::from("a"))
        );
        assert_eq!(
            IndexPacket::from(String::from("+")),
            IndexPacket::Insert(String::from(""))
        );
        assert_eq!(
            IndexPacket::from(String::from("-b")),
            IndexPacket::Remove(String::from("b"))
        );
        assert_eq!(
            IndexPacket::from(String::from("")),
            IndexPacket::Snapshot(vec![])
        );
        assert_eq!(
            IndexPacket::from(String::from("a\nb\nc\n")),
            IndexPacket::Snapshot(vec![
                String::from("a"),
                String::from("b"),
                String::from("c")
            ])
        );
        assert_eq!(
            IndexPacket::from(String::from("a\n")),
            IndexPacket::Snapshot(vec![String::from("a")])
        );
        assert_eq!(
            IndexPacket::from(String::from(
                "-avjVnd-ZhAhSiWwcrSBZ\naM7kvaj-Y1LnFZ4krInja\n"
            )),
            IndexPacket::Snapshot(vec![
                String::from("-avjVnd-ZhAhSiWwcrSBZ"),
                String::from("aM7kvaj-Y1LnFZ4krInja")
            ])
        );
    }
}
