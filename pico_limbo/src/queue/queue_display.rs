use crate::configuration::queue::{
    QueueActionBarConfig, QueueBossBarConfig, QueueConfig, QueueTabListConfig, QueueTitleConfig,
};
use crate::queue::placeholder::apply_placeholders;
use crate::server::packet_registry::PacketRegistry;
use minecraft_packets::play::boss_bar_packet::{BossBarColor, BossBarDivision, BossBarPacket};
use minecraft_packets::play::set_action_bar_text_packet::SetActionBarTextPacket;
use minecraft_packets::play::set_subtitle_text_packet::SetSubtitleTextPacket;
use minecraft_packets::play::set_title_text_packet::SetTitleTextPacket;
use minecraft_packets::play::set_titles_animation::SetTitlesAnimationPacket;
use minecraft_packets::play::tab_list_packet::TabListPacket;
use minecraft_protocol::prelude::{ProtocolVersion, Uuid};
use net::raw_packet::RawPacket;
use pico_text_component::prelude::parse_mini_message;

fn encode(packet: PacketRegistry, protocol_version: ProtocolVersion) -> Option<RawPacket> {
    packet.encode_packet(protocol_version).ok()
}

pub fn build_display_packets(
    config: &QueueConfig,
    position: usize,
    total: usize,
    username: &str,
    protocol_version: ProtocolVersion,
    boss_bar_uuid: Uuid,
) -> Vec<RawPacket> {
    let mut packets = Vec::new();

    let push_interval = config.push_interval_seconds;
    let push_count = config.push_count;

    // Tab list
    if let QueueTabListConfig::Enabled(ref tab_cfg) = config.tab_list {
        let header_text =
            apply_placeholders(&tab_cfg.header, position, total, username, push_interval, push_count);
        let footer_text =
            apply_placeholders(&tab_cfg.footer, position, total, username, push_interval, push_count);
        if let (Ok(header), Ok(footer)) = (
            parse_mini_message(&header_text),
            parse_mini_message(&footer_text),
        ) {
            let packet = TabListPacket::new(&header, &footer);
            if let Some(raw) = encode(PacketRegistry::TabList(packet), protocol_version) {
                packets.push(raw);
            }
        }
    }

    // Title (only for 1.8+)
    if protocol_version.is_after_inclusive(ProtocolVersion::V1_8) {
        if let QueueTitleConfig::Enabled(ref title_cfg) = config.title {
            let title_text = apply_placeholders(
                &title_cfg.title,
                position,
                total,
                username,
                push_interval,
                push_count,
            );
            let subtitle_text = apply_placeholders(
                &title_cfg.subtitle,
                position,
                total,
                username,
                push_interval,
                push_count,
            );

            if protocol_version.is_after_inclusive(ProtocolVersion::V1_17) {
                let anim = SetTitlesAnimationPacket::new(
                    title_cfg.fade_in,
                    title_cfg.stay,
                    title_cfg.fade_out,
                );
                if let Some(raw) =
                    encode(PacketRegistry::SetTitlesAnimation(anim), protocol_version)
                {
                    packets.push(raw);
                }
                if let Ok(comp) = parse_mini_message(&title_text) {
                    let pkt = SetTitleTextPacket::new(&comp);
                    if let Some(raw) = encode(PacketRegistry::SetTitleText(pkt), protocol_version) {
                        packets.push(raw);
                    }
                }
                if let Ok(comp) = parse_mini_message(&subtitle_text) {
                    let pkt = SetSubtitleTextPacket::new(&comp);
                    if let Some(raw) =
                        encode(PacketRegistry::SetSubtitleText(pkt), protocol_version)
                    {
                        packets.push(raw);
                    }
                }
            } else {
                use minecraft_packets::play::legacy_set_title_packet::LegacySetTitlePacket;
                let anim = LegacySetTitlePacket::set_animation(
                    title_cfg.fade_in,
                    title_cfg.stay,
                    title_cfg.fade_out,
                );
                if let Some(raw) = encode(PacketRegistry::LegacySetTitle(anim), protocol_version) {
                    packets.push(raw);
                }
                if let Ok(comp) = parse_mini_message(&title_text) {
                    let pkt = LegacySetTitlePacket::set_title(&comp);
                    if let Some(raw) =
                        encode(PacketRegistry::LegacySetTitle(pkt), protocol_version)
                    {
                        packets.push(raw);
                    }
                }
                if let Ok(comp) = parse_mini_message(&subtitle_text) {
                    let pkt = LegacySetTitlePacket::set_subtitle(&comp);
                    if let Some(raw) =
                        encode(PacketRegistry::LegacySetTitle(pkt), protocol_version)
                    {
                        packets.push(raw);
                    }
                }
            }
        }
    }

    // Action bar (1.8+)
    if protocol_version.is_after_inclusive(ProtocolVersion::V1_8) {
        if let QueueActionBarConfig::Enabled(ref ab_cfg) = config.action_bar {
            let text = apply_placeholders(
                &ab_cfg.text,
                position,
                total,
                username,
                push_interval,
                push_count,
            );
            if let Ok(comp) = parse_mini_message(&text) {
                if protocol_version.is_after_inclusive(ProtocolVersion::V1_17) {
                    let pkt = SetActionBarTextPacket::new(&comp);
                    if let Some(raw) =
                        encode(PacketRegistry::SetActionBarText(pkt), protocol_version)
                    {
                        packets.push(raw);
                    }
                } else if protocol_version.is_after_inclusive(ProtocolVersion::V1_11) {
                    use minecraft_packets::play::legacy_set_title_packet::LegacySetTitlePacket;
                    let pkt = LegacySetTitlePacket::action_bar(&comp);
                    if let Some(raw) =
                        encode(PacketRegistry::LegacySetTitle(pkt), protocol_version)
                    {
                        packets.push(raw);
                    }
                } else {
                    use minecraft_packets::play::legacy_chat_message_packet::LegacyChatMessagePacket;
                    let pkt = LegacyChatMessagePacket::game_info(&comp);
                    if let Some(raw) =
                        encode(PacketRegistry::LegacyChatMessage(pkt), protocol_version)
                    {
                        packets.push(raw);
                    }
                }
            }
        }
    }

    // Boss bar (1.9+)
    if protocol_version.is_after_inclusive(ProtocolVersion::V1_9) {
        if let QueueBossBarConfig::Enabled(ref bb_cfg) = config.boss_bar {
            let title_text = apply_placeholders(
                &bb_cfg.title,
                position,
                total,
                username,
                push_interval,
                push_count,
            );
            // health = 1.0 - (position - 1) / total (drains as you move up the queue)
            let health = if total == 0 {
                1.0_f32
            } else {
                1.0_f32 - (position.saturating_sub(1) as f32 / total as f32)
            };
            if let Ok(comp) = parse_mini_message(&title_text) {
                let color: BossBarColor = bb_cfg.color.into();
                let division: BossBarDivision = bb_cfg.division.into();
                // First remove, then re-add with updated info
                let remove_pkt = BossBarPacket::remove(boss_bar_uuid);
                if let Some(raw) = encode(PacketRegistry::BossBar(remove_pkt), protocol_version) {
                    packets.push(raw);
                }
                let add_pkt =
                    BossBarPacket::add_with_uuid(boss_bar_uuid, &comp, health, color, division);
                if let Some(raw) = encode(PacketRegistry::BossBar(add_pkt), protocol_version) {
                    packets.push(raw);
                }
            }
        }
    }

    packets
}

pub fn build_boss_bar_remove_packet(
    config: &QueueConfig,
    boss_bar_uuid: Uuid,
    protocol_version: ProtocolVersion,
) -> Option<RawPacket> {
    if protocol_version.is_after_inclusive(ProtocolVersion::V1_9) {
        if let QueueBossBarConfig::Enabled(_) = config.boss_bar {
            let pkt = BossBarPacket::remove(boss_bar_uuid);
            return encode(PacketRegistry::BossBar(pkt), protocol_version);
        }
    }
    None
}
