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

/// Build packets to send on every action-bar tick (every ~1s).
/// Only includes the action bar packet.
pub fn build_action_bar_packet(
    config: &QueueConfig,
    position: usize,
    total: usize,
    username: &str,
    protocol_version: ProtocolVersion,
) -> Option<RawPacket> {
    if !protocol_version.is_after_inclusive(ProtocolVersion::V1_8) {
        return None;
    }
    let QueueActionBarConfig::Enabled(ref ab_cfg) = config.action_bar else {
        return None;
    };
    let push_interval = config.push_interval_seconds;
    let push_count = config.push_count;
    let text = apply_placeholders(&ab_cfg.text, position, total, username, push_interval, push_count);
    let comp = parse_mini_message(&text).ok()?;
    if protocol_version.is_after_inclusive(ProtocolVersion::V1_17) {
        let pkt = SetActionBarTextPacket::new(&comp);
        encode(PacketRegistry::SetActionBarText(pkt), protocol_version)
    } else if protocol_version.is_after_inclusive(ProtocolVersion::V1_11) {
        use minecraft_packets::play::legacy_set_title_packet::LegacySetTitlePacket;
        let pkt = LegacySetTitlePacket::action_bar(&comp);
        encode(PacketRegistry::LegacySetTitle(pkt), protocol_version)
    } else {
        use minecraft_packets::play::legacy_chat_message_packet::LegacyChatMessagePacket;
        let pkt = LegacyChatMessagePacket::game_info(&comp);
        encode(PacketRegistry::LegacyChatMessage(pkt), protocol_version)
    }
}

/// Build packets sent on each full refresh tick (tab list, title, boss bar).
/// `boss_bar_initialized` should be `false` on the very first call so the boss bar
/// is added; `true` on subsequent calls so only update packets are sent (no flicker).
pub fn build_display_packets(
    config: &QueueConfig,
    position: usize,
    total: usize,
    username: &str,
    protocol_version: ProtocolVersion,
    boss_bar_uuid: Uuid,
    boss_bar_initialized: bool,
) -> Vec<RawPacket> {
    let mut packets = Vec::new();

    let push_interval = config.push_interval_seconds;
    let push_count = config.push_count;

    // Tab list
    if let QueueTabListConfig::Enabled(ref tab_cfg) = config.tab_list {
        let raw_header = tab_cfg.header.join("\n");
        let raw_footer = tab_cfg.footer.join("\n");
        let header_text =
            apply_placeholders(&raw_header, position, total, username, push_interval, push_count);
        let footer_text =
            apply_placeholders(&raw_footer, position, total, username, push_interval, push_count);
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

    // Boss bar (1.9+) — add once, then use update packets to avoid flicker
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
            // health drains as you move up the queue
            let health = if total == 0 {
                1.0_f32
            } else {
                1.0_f32 - (position.saturating_sub(1) as f32 / total as f32)
            };
            if let Ok(comp) = parse_mini_message(&title_text) {
                let color: BossBarColor = bb_cfg.color.into();
                let division: BossBarDivision = bb_cfg.division.into();
                if !boss_bar_initialized {
                    // First time: send Add
                    let add_pkt = BossBarPacket::add_with_uuid(
                        boss_bar_uuid,
                        &comp,
                        health,
                        color,
                        division,
                    );
                    if let Some(raw) =
                        encode(PacketRegistry::BossBar(add_pkt), protocol_version)
                    {
                        packets.push(raw);
                    }
                } else {
                    // Subsequent refreshes: update title and health in-place (no flicker)
                    let update_title = BossBarPacket::update_title(boss_bar_uuid, &comp);
                    if let Some(raw) =
                        encode(PacketRegistry::BossBar(update_title), protocol_version)
                    {
                        packets.push(raw);
                    }
                    let update_health = BossBarPacket::update_health(boss_bar_uuid, health);
                    if let Some(raw) =
                        encode(PacketRegistry::BossBar(update_health), protocol_version)
                    {
                        packets.push(raw);
                    }
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
