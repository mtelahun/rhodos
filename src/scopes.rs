// These are specific to mastodon "scopes" parameter in client registration API. They
// shouldn't be confused with OAuth "scope" values used when interacting with mastodon
// OAuth APIs.
//
pub const GLOBAL_READ: &str = "read";
pub const GLOBAL_WRITE: &str = "write";
// follow has been deprecated since mastodon ver. 3.5.0
pub const GLOBAL_FOLLOW: &str = "follow";

pub const FOLLOW_SCOPES: &[&str] = &[
    Blocks::READ,
    Blocks::WRITE,
    Follows::READ,
    Follows::WRITE,
    Mutes::READ,
    Mutes::WRITE,
];

pub const READ_SCOPES: &[&str] = &[
    Accounts::READ,
    Blocks::READ,
    Bookmarks::READ,
    Favourites::READ,
    Filters::READ,
    Follows::READ,
    Lists::READ,
    Mutes::READ,
    Notifications::READ,
    Search::READ,
    Statuses::READ,
];

pub const WRITE_SCOPES: &[&str] = &[
    Accounts::WRITE,
    Blocks::WRITE,
    Bookmarks::WRITE,
    Conversations::WRITE,
    Favourites::WRITE,
    Filters::WRITE,
    Follows::WRITE,
    Media::WRITE,
    Lists::WRITE,
    Mutes::WRITE,
    Notifications::WRITE,
    Reports::WRITE,
    Statuses::WRITE,
];

pub const SCOPES: &[&str] = &[
    Accounts::READ,
    Accounts::WRITE,
    Blocks::READ,
    Blocks::WRITE,
    Bookmarks::READ,
    Bookmarks::WRITE,
    Conversations::WRITE,
    Favourites::READ,
    Favourites::WRITE,
    Filters::READ,
    Filters::WRITE,
    Follows::READ,
    Follows::WRITE,
    Media::WRITE,
    Lists::READ,
    Lists::WRITE,
    Mutes::READ,
    Mutes::WRITE,
    Notifications::READ,
    Notifications::WRITE,
    Reports::WRITE,
    Search::READ,
    Statuses::READ,
    Statuses::WRITE,
];

pub trait Permission {
    const READ: &'static str;
    const WRITE: &'static str;
}

pub struct Accounts;
pub struct Blocks;
pub struct Bookmarks;
pub struct Conversations;
pub struct Favourites;
pub struct Filters;
pub struct Follows;
pub struct Lists;
pub struct Media;
pub struct Mutes;
pub struct Notifications;
pub struct Reports;
pub struct Search;
pub struct Statuses;

impl Permission for Accounts {
    const READ: &'static str = "read:accounts";
    const WRITE: &'static str = "write:accounts";
}

impl Permission for Blocks {
    const READ: &'static str = "read:blocks";
    const WRITE: &'static str = "write:blocks";
}

impl Permission for Bookmarks {
    const READ: &'static str = "read:bookmarks";
    const WRITE: &'static str = "write:bookmarks";
}

impl Permission for Conversations {
    const READ: &'static str = "--";
    const WRITE: &'static str = "write:conversations";
}

impl Permission for Favourites {
    const READ: &'static str = "read:favourites";
    const WRITE: &'static str = "write:favourites";
}

impl Permission for Filters {
    const READ: &'static str = "read:filters";
    const WRITE: &'static str = "write:filters";
}

impl Permission for Follows {
    const READ: &'static str = "read:follows";
    const WRITE: &'static str = "write:follows";
}

impl Permission for Lists {
    const READ: &'static str = "read:lists";
    const WRITE: &'static str = "write:lists";
}

impl Permission for Media {
    const READ: &'static str = "--";
    const WRITE: &'static str = "write:media";
}

impl Permission for Mutes {
    const READ: &'static str = "read:mutes";
    const WRITE: &'static str = "write:mutes";
}

impl Permission for Notifications {
    const READ: &'static str = "read:notifications";
    const WRITE: &'static str = "write:notifications";
}

impl Permission for Reports {
    const READ: &'static str = "--";
    const WRITE: &'static str = "write:reports";
}

impl Permission for Search {
    const READ: &'static str = "read:search";
    const WRITE: &'static str = "--";
}

impl Permission for Statuses {
    const READ: &'static str = "read:statuses";
    const WRITE: &'static str = "write:statuses";
}

#[derive(Debug)]
pub enum Scopes {
    ReadAccounts,
    ReadBlocks,
    ReadBookmarks,
    ReadFavourites,
    ReadFilters,
    ReadFollows,
    ReadLists,
    ReadMutes,
    ReadNotifications,
    ReadSearch,
    ReadStatuses,
    WriteAccounts,
    WriteBlocks,
    WriteBookmarks,
    WriteConversations,
    WriteFavouries,
    WriteFilters,
    WriteFollows,
    WriteLists,
    WriteMedia,
    WriteMutes,
    WriteNotifications,
    WriteReports,
    WriteStatuses,
}

impl std::str::FromStr for Scopes {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            Accounts::READ => Self::ReadAccounts,
            Accounts::WRITE => Self::WriteAccounts,
            Blocks::READ => Self::ReadBlocks,
            Blocks::WRITE => Self::WriteBlocks,
            Bookmarks::READ => Self::ReadBookmarks,
            Bookmarks::WRITE => Self::WriteBookmarks,
            Conversations::WRITE => Self::WriteConversations,
            Favourites::READ => Self::ReadFavourites,
            Favourites::WRITE => Self::WriteFavouries,
            Filters::READ => Self::ReadFilters,
            Filters::WRITE => Self::WriteFilters,
            Follows::READ => Self::ReadFollows,
            Follows::WRITE => Self::WriteFollows,
            Lists::READ => Self::ReadLists,
            Lists::WRITE => Self::WriteLists,
            Media::WRITE => Self::WriteMedia,
            Mutes::READ => Self::ReadMutes,
            Mutes::WRITE => Self::WriteMutes,
            Notifications::READ => Self::ReadNotifications,
            Notifications::WRITE => Self::WriteNotifications,
            Reports::WRITE => Self::WriteReports,
            Search::READ => Self::ReadSearch,
            Statuses::READ => Self::ReadStatuses,
            Statuses::WRITE => Self::WriteStatuses,
            _ => return Err(()),
        })
    }
}

pub struct Read<S>(pub S);
pub struct Write<S>(pub S);

pub trait Scope {
    const SCOPE: &'static str;
}

impl Scope for () {
    const SCOPE: &'static str = "";
}

impl<S: Permission> Scope for Read<S> {
    const SCOPE: &'static str = S::READ;
}

impl<S: Permission> Scope for Write<S> {
    const SCOPE: &'static str = S::WRITE;
}
