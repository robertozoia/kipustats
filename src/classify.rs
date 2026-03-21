use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientType {
    Browser,
    RssReader,
    Bot,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientInfo {
    pub client_type: ClientType,
    pub client_name: String,
}

const RSS_READERS: &[(&str, &str)] = &[
    ("feedly", "Feedly"),
    ("inoreader", "Inoreader"),
    ("newsblur", "NewsBlur"),
    ("miniflux", "Miniflux"),
    ("netnewswire", "NetNewsWire"),
    ("freshreader", "FreshReader"),
    ("theoldreader", "The Old Reader"),
    ("bazqux", "BazQux"),
    ("feedbin", "Feedbin"),
    ("tiny tiny rss", "Tiny Tiny RSS"),
    ("liferea", "Liferea"),
    ("newsboat", "Newsboat"),
    ("reeder", "Reeder"),
    ("readkit", "ReadKit"),
    ("feedreader", "FeedReader"),
    ("rssowl", "RSSOwl"),
    ("akregator", "Akregator"),
    ("yarr", "Yarr"),
    ("thunderbird", "Thunderbird"),
    // Generic fallbacks — must come last
    ("rss", "RSS Reader (other)"),
    ("feed", "Feed Reader (other)"),
    ("atom", "Feed Reader (other)"),
];

const BOTS: &[(&str, &str)] = &[
    ("googlebot", "Googlebot"),
    ("bingbot", "Bingbot"),
    ("yandexbot", "YandexBot"),
    ("baiduspider", "Baiduspider"),
    ("duckduckbot", "DuckDuckBot"),
    ("gptbot", "GPTBot"),
    ("chatgpt-user", "ChatGPT-User"),
    ("claudebot", "ClaudeBot"),
    ("anthropic-ai", "Anthropic AI"),
    ("ccbot", "CCBot"),
    ("facebookexternalhit", "Facebook"),
    ("twitterbot", "Twitter"),
    ("linkedinbot", "LinkedIn"),
    ("slackbot", "Slackbot"),
    ("telegrambot", "TelegramBot"),
    ("discordbot", "DiscordBot"),
    ("applebot", "Applebot"),
    ("semrushbot", "SemrushBot"),
    ("ahrefsbot", "AhrefsBot"),
    ("mj12bot", "Majestic"),
    ("dotbot", "DotBot"),
    ("petalbot", "PetalBot"),
    ("bytespider", "ByteSpider"),
    ("amazonbot", "Amazonbot"),
    ("seznam", "SeznamBot"),
    ("ia_archiver", "Alexa"),
    ("archive.org_bot", "Internet Archive"),
    // Generic fallbacks — must come last
    ("bot", "Bot (other)"),
    ("spider", "Bot (other)"),
    ("crawler", "Bot (other)"),
    ("scraper", "Bot (other)"),
];

pub fn classify(user_agent: &str) -> ClientInfo {
    let ua_lower = user_agent.to_ascii_lowercase();

    // Check RSS readers first
    for &(pattern, name) in RSS_READERS {
        if ua_lower.contains(pattern) {
            return ClientInfo {
                client_type: ClientType::RssReader,
                client_name: name.to_string(),
            };
        }
    }

    // Check bots
    for &(pattern, name) in BOTS {
        if ua_lower.contains(pattern) {
            return ClientInfo {
                client_type: ClientType::Bot,
                client_name: name.to_string(),
            };
        }
    }

    // Default: browser
    let client_name = extract_browser_name(&ua_lower);
    ClientInfo {
        client_type: ClientType::Browser,
        client_name,
    }
}

fn extract_browser_name(ua_lower: &str) -> String {
    // Order matters: check more specific strings before generic ones
    if ua_lower.contains("edg/") || ua_lower.contains("edge/") {
        "Edge".to_string()
    } else if ua_lower.contains("opr/") || ua_lower.contains("opera") {
        "Opera".to_string()
    } else if ua_lower.contains("vivaldi") {
        "Vivaldi".to_string()
    } else if ua_lower.contains("brave") {
        "Brave".to_string()
    } else if ua_lower.contains("firefox") {
        "Firefox".to_string()
    } else if ua_lower.contains("chrome") || ua_lower.contains("chromium") {
        "Chrome".to_string()
    } else if ua_lower.contains("safari") {
        "Safari".to_string()
    } else if ua_lower.contains("curl") {
        "curl".to_string()
    } else if ua_lower.contains("wget") {
        "wget".to_string()
    } else {
        "Unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_readers() {
        let cases = vec![
            ("Feedly/1.0 (+http://www.feedly.com/fetcher.html; like FeedFetcher-Google)", "Feedly", ClientType::RssReader),
            ("Inoreader/1.0", "Inoreader", ClientType::RssReader),
            ("NewsBlur Feed Fetcher - 42 subscribers", "NewsBlur", ClientType::RssReader),
            ("Miniflux/2.0.50", "Miniflux", ClientType::RssReader),
            ("NetNewsWire (RSS Reader; https://netnewswire.com/)", "NetNewsWire", ClientType::RssReader),
            ("Newsboat/2.30", "Newsboat", ClientType::RssReader),
            ("Reeder/5.0", "Reeder", ClientType::RssReader),
            ("Thunderbird/115.0", "Thunderbird", ClientType::RssReader),
        ];

        for (ua, expected_name, expected_type) in cases {
            let info = classify(ua);
            assert_eq!(info.client_type, expected_type, "Failed for UA: {ua}");
            assert_eq!(info.client_name, expected_name, "Failed for UA: {ua}");
        }
    }

    #[test]
    fn test_bots() {
        let cases = vec![
            ("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)", "Googlebot"),
            ("Mozilla/5.0 (compatible; bingbot/2.0; +http://www.bing.com/bingbot.htm)", "Bingbot"),
            ("Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; GPTBot/1.0)", "GPTBot"),
            ("claudebot", "ClaudeBot"),
            ("facebookexternalhit/1.1", "Facebook"),
            ("Slackbot-LinkExpanding 1.0", "Slackbot"),
            ("AhrefsBot/7.0", "AhrefsBot"),
            ("SomeRandomCrawler/1.0", "Bot (other)"),
        ];

        for (ua, expected_name) in cases {
            let info = classify(ua);
            assert_eq!(info.client_type, ClientType::Bot, "Failed for UA: {ua}");
            assert_eq!(info.client_name, expected_name, "Failed for UA: {ua}");
        }
    }

    #[test]
    fn test_browsers() {
        let cases = vec![
            ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36", "Chrome"),
            ("Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0", "Firefox"),
            ("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15", "Safari"),
            ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0", "Edge"),
        ];

        for (ua, expected_name) in cases {
            let info = classify(ua);
            assert_eq!(info.client_type, ClientType::Browser, "Failed for UA: {ua}");
            assert_eq!(info.client_name, expected_name, "Failed for UA: {ua}");
        }
    }

    #[test]
    fn test_generic_rss_fallback() {
        let info = classify("SomeApp/1.0 RSS");
        assert_eq!(info.client_type, ClientType::RssReader);
        assert_eq!(info.client_name, "RSS Reader (other)");
    }
}
