//! Seed data definitions.
//!
//! Obsahuje predpripravené vzorové dáta v slovenčine pre organizáciu, používateľov,
//! budovy, jednotky, poruchy, hlasovania, oznámenia, inzeráty a realitný portál.

/// Seed dáta pre organizáciu.
#[derive(Debug, Clone)]
pub struct SeedOrganization {
    pub name: &'static str,
    pub slug: &'static str,
    pub contact_email: &'static str,
}

/// Seed dáta pre budovu.
#[derive(Debug, Clone)]
pub struct SeedBuilding {
    pub name: &'static str,
    pub street: &'static str,
    pub city: &'static str,
    pub postal_code: &'static str,
    pub country: &'static str,
    pub total_floors: i32,
    pub year_built: Option<i32>,
    pub units: Vec<SeedUnit>,
}

/// Seed dáta pre bytovú jednotku.
#[derive(Debug, Clone)]
pub struct SeedUnit {
    pub designation: &'static str,
    pub floor: i32,
    pub unit_type: &'static str,
    pub size_sqm: Option<i32>,
    pub rooms: Option<i32>,
}

/// Seed dáta pre používateľa.
#[derive(Debug, Clone)]
pub struct SeedUser {
    pub email: &'static str,
    pub name: &'static str,
    pub role_type: &'static str,
    pub phone: Option<&'static str>,
    pub locale: &'static str,
    /// Priradenia jednotiek pre tohto používateľa
    pub unit_assignments: Vec<SeedUnitAssignment>,
}

/// Priradenie používateľa k jednotke.
#[derive(Debug, Clone)]
pub struct SeedUnitAssignment {
    /// Index do poľa buildings
    pub building_index: usize,
    /// Index do poľa units v rámci budovy
    pub unit_index: usize,
    /// Typ obyvateľa: owner, tenant, family_member, subtenant
    pub resident_type: &'static str,
    /// Či ide o primárneho obyvateľa
    pub is_primary: bool,
}

/// Seed dáta pre poruchu.
#[derive(Debug, Clone)]
pub struct SeedFault {
    pub title: &'static str,
    pub description: &'static str,
    pub location_description: Option<&'static str>,
    pub category: &'static str,
    pub priority: &'static str,
    pub status: &'static str,
    pub building_index: usize,
    /// Voliteľný index jednotky v rámci budovy; None = spoločný priestor
    pub unit_index: Option<usize>,
    /// Index do seed_data.users
    pub reporter_user_index: usize,
    /// Index do seed_data.users — použitý aj ako triaged_by
    pub assigned_user_index: Option<usize>,
    /// Index do seed_data.users
    pub resolved_by_user_index: Option<usize>,
    /// Index do seed_data.users (spravidla nahlasovateľ)
    pub confirmed_by_user_index: Option<usize>,
    pub rating: Option<i32>,
}

/// Seed dáta pre otázku hlasovania.
#[derive(Debug, Clone)]
pub struct SeedVoteQuestion {
    pub question_text: &'static str,
    /// "yes_no" | "single_choice"
    pub question_type: &'static str,
    /// Možnosti odpovede pre single_choice (pre yes_no nechajte prázdne)
    pub options: &'static [&'static str],
}

/// Seed dáta pre hlasovanie.
#[derive(Debug, Clone)]
pub struct SeedVote {
    pub title: &'static str,
    pub description: Option<&'static str>,
    pub building_index: usize,
    /// "draft" | "active" | "closed"
    pub status: &'static str,
    /// Index do seed_data.users (vytvorí a zverejní hlasovanie)
    pub created_by_user_index: usize,
    /// "simple_majority" | "two_thirds"
    pub quorum_type: &'static str,
    pub questions: Vec<SeedVoteQuestion>,
}

/// Seed dáta pre oznámenie.
#[derive(Debug, Clone)]
pub struct SeedAnnouncement {
    pub title: &'static str,
    pub content: &'static str,
    /// "all" | "building"
    pub target_type: &'static str,
    /// Použité keď target_type = "building"
    pub building_index: Option<usize>,
    /// "draft" | "published" | "archived"
    pub status: &'static str,
    pub pinned: bool,
    pub acknowledgment_required: bool,
    /// Index do seed_data.users
    pub author_user_index: usize,
}

/// Seed dáta pre inzerát (PPT → Reality portál).
#[derive(Debug, Clone)]
pub struct SeedListing {
    pub title: &'static str,
    pub description: &'static str,
    /// "sale" | "rent"
    pub transaction_type: &'static str,
    /// "apartment" | "commercial" | "parking" | "storage"
    pub property_type: &'static str,
    /// "draft" | "active" | "archived"
    pub status: &'static str,
    pub building_index: usize,
    /// Voliteľný index jednotky — None = manuálne zadaná adresa
    pub unit_index: Option<usize>,
    /// Index do seed_data.users (PPT používateľ, ktorý vytvoril inzerát)
    pub created_by_user_index: usize,
    /// Cena v EUR (predaj) alebo EUR/mesiac (prenájom)
    pub price: i64,
    pub is_negotiable: bool,
    pub rooms: Option<i32>,
    pub bathrooms: Option<i32>,
    /// Vlastnosti inzerátu (napr. "Výťah", "Parkovanie")
    pub features: &'static [&'static str],
}

/// Seed dáta pre používateľa realitného portálu.
/// Títo používatelia sú oddelení od PPT používateľov (portal_users tabuľka).
#[derive(Debug, Clone)]
pub struct SeedPortalUser {
    pub email: &'static str,
    pub name: &'static str,
    pub phone: Option<&'static str>,
    /// Ak je Some(index), prepojí sa na seed_data.users[index] cez pm_user_id
    pub pm_user_index: Option<usize>,
}

/// Seed dáta pre dopyt záujemcu o inzerát.
#[derive(Debug, Clone)]
pub struct SeedInquiry {
    /// Index do seed_data.listings
    pub listing_index: usize,
    /// Index do seed_data.portal_users — záujemca
    pub portal_user_index: usize,
    /// Index do seed_data.portal_users — realitný maklér (príjemca)
    pub realtor_portal_user_index: usize,
    pub message: &'static str,
    /// "info" | "viewing" | "offer"
    pub inquiry_type: &'static str,
    /// "email" | "phone"
    pub preferred_contact: &'static str,
    /// "new" | "read" | "responded"
    pub status: &'static str,
    /// Odpoveď makléra (None ak status nie je "responded")
    pub reply_message: Option<&'static str>,
}

/// Seed dáta pre obľúbený inzerát portálového používateľa.
#[derive(Debug, Clone)]
pub struct SeedPortalFavorite {
    /// Index do seed_data.portal_users
    pub portal_user_index: usize,
    /// Index do seed_data.listings
    pub listing_index: usize,
}

/// Kompletná konfigurácia seed dát.
#[derive(Debug, Clone)]
pub struct SeedData {
    pub organization: SeedOrganization,
    pub buildings: Vec<SeedBuilding>,
    pub users: Vec<SeedUser>,
    pub faults: Vec<SeedFault>,
    pub votes: Vec<SeedVote>,
    pub announcements: Vec<SeedAnnouncement>,
    pub listings: Vec<SeedListing>,
    pub portal_users: Vec<SeedPortalUser>,
    pub inquiries: Vec<SeedInquiry>,
    pub portal_favorites: Vec<SeedPortalFavorite>,
    /// Predvolené heslo pre vzorových používateľov
    pub default_password: &'static str,
    /// Predvolené heslo pre portálových používateľov
    pub portal_default_password: &'static str,
}

impl Default for SeedData {
    fn default() -> Self {
        Self {
            organization: SeedOrganization {
                name: "Demo správa nehnuteľností",
                slug: "demo-property",
                contact_email: "info@demo-property.test",
            },
            buildings: vec![
                // Budova 0: Bytový dom Hlavná (8 jednotiek)
                SeedBuilding {
                    name: "Bytový dom Hlavná",
                    street: "Hlavná 123",
                    city: "Bratislava",
                    postal_code: "81101",
                    country: "SK",
                    total_floors: 5,
                    year_built: Some(2010),
                    units: vec![
                        SeedUnit {
                            designation: "1A",
                            floor: 1,
                            unit_type: "apartment",
                            size_sqm: Some(65),
                            rooms: Some(2),
                        },
                        SeedUnit {
                            designation: "1B",
                            floor: 1,
                            unit_type: "apartment",
                            size_sqm: Some(75),
                            rooms: Some(3),
                        },
                        SeedUnit {
                            designation: "2A",
                            floor: 2,
                            unit_type: "apartment",
                            size_sqm: Some(65),
                            rooms: Some(2),
                        },
                        SeedUnit {
                            designation: "2B",
                            floor: 2,
                            unit_type: "apartment",
                            size_sqm: Some(75),
                            rooms: Some(3),
                        },
                        SeedUnit {
                            designation: "3A",
                            floor: 3,
                            unit_type: "apartment",
                            size_sqm: Some(85),
                            rooms: Some(4),
                        },
                        SeedUnit {
                            designation: "P1",
                            floor: -1,
                            unit_type: "parking",
                            size_sqm: Some(15),
                            rooms: None,
                        },
                        SeedUnit {
                            designation: "P2",
                            floor: -1,
                            unit_type: "parking",
                            size_sqm: Some(15),
                            rooms: None,
                        },
                        SeedUnit {
                            designation: "S1",
                            floor: -1,
                            unit_type: "storage",
                            size_sqm: Some(5),
                            rooms: None,
                        },
                    ],
                },
                // Budova 1: Dubová rezidencia (6 jednotiek)
                SeedBuilding {
                    name: "Dubová rezidencia",
                    street: "Dubová 45",
                    city: "Bratislava",
                    postal_code: "82105",
                    country: "SK",
                    total_floors: 3,
                    year_built: Some(2015),
                    units: vec![
                        SeedUnit {
                            designation: "101",
                            floor: 1,
                            unit_type: "apartment",
                            size_sqm: Some(55),
                            rooms: Some(2),
                        },
                        SeedUnit {
                            designation: "102",
                            floor: 1,
                            unit_type: "apartment",
                            size_sqm: Some(55),
                            rooms: Some(2),
                        },
                        SeedUnit {
                            designation: "201",
                            floor: 2,
                            unit_type: "apartment",
                            size_sqm: Some(70),
                            rooms: Some(3),
                        },
                        SeedUnit {
                            designation: "202",
                            floor: 2,
                            unit_type: "apartment",
                            size_sqm: Some(70),
                            rooms: Some(3),
                        },
                        SeedUnit {
                            designation: "301",
                            floor: 3,
                            unit_type: "apartment",
                            size_sqm: Some(90),
                            rooms: Some(4),
                        },
                        SeedUnit {
                            designation: "G01",
                            floor: 0,
                            unit_type: "storage",
                            size_sqm: Some(8),
                            rooms: None,
                        },
                    ],
                },
                // Budova 2: Centrálna plaza (5 jednotiek, zmiešané využitie)
                SeedBuilding {
                    name: "Centrálna plaza",
                    street: "Centrálna 1",
                    city: "Bratislava",
                    postal_code: "81102",
                    country: "SK",
                    total_floors: 4,
                    year_built: Some(2020),
                    units: vec![
                        SeedUnit {
                            designation: "G1",
                            floor: 0,
                            unit_type: "commercial",
                            size_sqm: Some(120),
                            rooms: None,
                        },
                        SeedUnit {
                            designation: "G2",
                            floor: 0,
                            unit_type: "commercial",
                            size_sqm: Some(80),
                            rooms: None,
                        },
                        SeedUnit {
                            designation: "A1",
                            floor: 1,
                            unit_type: "apartment",
                            size_sqm: Some(100),
                            rooms: Some(4),
                        },
                        SeedUnit {
                            designation: "A2",
                            floor: 2,
                            unit_type: "apartment",
                            size_sqm: Some(100),
                            rooms: Some(4),
                        },
                        SeedUnit {
                            designation: "PH",
                            floor: 3,
                            unit_type: "apartment",
                            size_sqm: Some(150),
                            rooms: Some(5),
                        },
                    ],
                },
            ],
            users: vec![
                // 0: Správca organizácie
                SeedUser {
                    email: "spravca@demo-property.test",
                    name: "Správca organizácie",
                    role_type: "Organization Admin",
                    phone: Some("+421900111001"),
                    locale: "sk",
                    unit_assignments: vec![],
                },
                // 1: Manažér budovy
                SeedUser {
                    email: "manazer@demo-property.test",
                    name: "Peter Manažér",
                    role_type: "Manager",
                    phone: Some("+421900111002"),
                    locale: "sk",
                    unit_assignments: vec![],
                },
                // 2: Technický manažér
                SeedUser {
                    email: "technik@demo-property.test",
                    name: "Lukáš Technik",
                    role_type: "Technical Manager",
                    phone: Some("+421900111003"),
                    locale: "sk",
                    unit_assignments: vec![],
                },
                // 3: Vlastník 1 — vlastní byt 1A + parkovacie miesto P1
                SeedUser {
                    email: "novakova@demo-property.test",
                    name: "Jana Nováková",
                    role_type: "Owner",
                    phone: Some("+421900222001"),
                    locale: "sk",
                    unit_assignments: vec![
                        SeedUnitAssignment {
                            building_index: 0,
                            unit_index: 0, // 1A
                            resident_type: "owner",
                            is_primary: true,
                        },
                        SeedUnitAssignment {
                            building_index: 0,
                            unit_index: 5, // P1
                            resident_type: "owner",
                            is_primary: true,
                        },
                    ],
                },
                // 4: Vlastník 2 — vlastní byt 2B, prenajíma ho
                SeedUser {
                    email: "horvath@demo-property.test",
                    name: "Peter Horváth",
                    role_type: "Owner",
                    phone: Some("+421900222002"),
                    locale: "sk",
                    unit_assignments: vec![SeedUnitAssignment {
                        building_index: 0,
                        unit_index: 3, // 2B
                        resident_type: "owner",
                        is_primary: true,
                    }],
                },
                // 5: Vlastník 3 — vlastní byt 101 v Dubovej rezidencii
                SeedUser {
                    email: "kovacova@demo-property.test",
                    name: "Mária Kováčová",
                    role_type: "Owner",
                    phone: Some("+421900222003"),
                    locale: "sk",
                    unit_assignments: vec![SeedUnitAssignment {
                        building_index: 1,
                        unit_index: 0, // 101
                        resident_type: "owner",
                        is_primary: true,
                    }],
                },
                // 6: Splnomocnenec vlastníka — zastupuje vlastníka 2
                SeedUser {
                    email: "delegat@demo-property.test",
                    name: "Martin Delegát",
                    role_type: "Owner Delegate",
                    phone: Some("+421900222004"),
                    locale: "sk",
                    unit_assignments: vec![],
                },
                // 7: Správca nehnuteľností — spravuje krátkodobé prenájmy
                SeedUser {
                    email: "spravca.nehnutelnosti@demo-property.test",
                    name: "Lucia Správcová",
                    role_type: "Property Manager",
                    phone: Some("+421900333001"),
                    locale: "sk",
                    unit_assignments: vec![],
                },
                // 8: Realitný maklér
                SeedUser {
                    email: "makler@demo-property.test",
                    name: "Tomáš Maklér",
                    role_type: "Real Estate Agent",
                    phone: Some("+421900333002"),
                    locale: "sk",
                    unit_assignments: vec![],
                },
                // 9: Nájomník 1 — prenajíma byt 2B v Bytovom dome Hlavná (od vlastníka 2)
                SeedUser {
                    email: "najomnik1@demo-property.test",
                    name: "Ján Nájomník",
                    role_type: "Tenant",
                    phone: Some("+421900444001"),
                    locale: "sk",
                    unit_assignments: vec![SeedUnitAssignment {
                        building_index: 0,
                        unit_index: 3, // 2B
                        resident_type: "tenant",
                        is_primary: true,
                    }],
                },
                // 10: Nájomníčka 2 — prenajíma byt 201 v Dubovej rezidencii
                SeedUser {
                    email: "najomnik2@demo-property.test",
                    name: "Eva Prenajímateľka",
                    role_type: "Tenant",
                    phone: Some("+421900444002"),
                    locale: "sk",
                    unit_assignments: vec![SeedUnitAssignment {
                        building_index: 1,
                        unit_index: 2, // 201
                        resident_type: "tenant",
                        is_primary: true,
                    }],
                },
                // 11: Nájomník 3 — prenajíma komerčný priestor G1 v Centrálnej plaze
                SeedUser {
                    email: "najomnik3@demo-property.test",
                    name: "Firma s.r.o.",
                    role_type: "Tenant",
                    phone: Some("+421900444003"),
                    locale: "sk",
                    unit_assignments: vec![SeedUnitAssignment {
                        building_index: 2,
                        unit_index: 0, // G1
                        resident_type: "tenant",
                        is_primary: true,
                    }],
                },
                // 12: Obyvateľ 1 — rodinný príslušník v byte 1A
                SeedUser {
                    email: "obyvatel1@demo-property.test",
                    name: "Michal Novák",
                    role_type: "Resident",
                    phone: Some("+421900555001"),
                    locale: "sk",
                    unit_assignments: vec![SeedUnitAssignment {
                        building_index: 0,
                        unit_index: 0, // 1A
                        resident_type: "family_member",
                        is_primary: false,
                    }],
                },
                // 13: Obyvateľka 2 — podnájomníčka v byte 201
                SeedUser {
                    email: "obyvatel2@demo-property.test",
                    name: "Anna Rezidentka",
                    role_type: "Resident",
                    phone: Some("+421900555002"),
                    locale: "sk",
                    unit_assignments: vec![SeedUnitAssignment {
                        building_index: 1,
                        unit_index: 2, // 201
                        resident_type: "subtenant",
                        is_primary: false,
                    }],
                },
                // 14: Hosť — dočasný prístup
                SeedUser {
                    email: "host@demo-property.test",
                    name: "Hosťovský používateľ",
                    role_type: "Guest",
                    phone: None,
                    locale: "sk",
                    unit_assignments: vec![],
                },
            ],
            faults: vec![
                // Porucha 0: Uzavretá porucha inštalácie v byte 1A
                SeedFault {
                    title: "Únik vody pod kuchynským drezom",
                    description: "Pod kuchynským drezom vytéka voda. Voda sa zbiera v skrinke a spôsobuje poškodenie dreva.",
                    location_description: Some("Kuchyňa, byt 1A"),
                    category: "plumbing",
                    priority: "urgent",
                    status: "closed",
                    building_index: 0,
                    unit_index: Some(0), // 1A
                    reporter_user_index: 3,  // novakova
                    assigned_user_index: Some(2), // technik
                    resolved_by_user_index: Some(2), // technik
                    confirmed_by_user_index: Some(3), // novakova
                    rating: Some(5),
                },
                // Porucha 1: Výťah nefunguje — prebieha oprava
                SeedFault {
                    title: "Výťah mimo prevádzky",
                    description: "Výťah vydával škrípavé zvuky a ráno sa úplne zastavil. Postihnutí sú obyvatelia s obmedzenou pohyblivosťou.",
                    location_description: Some("Výťahová šachta, prízemie"),
                    category: "elevator",
                    priority: "high",
                    status: "in_progress",
                    building_index: 0,
                    unit_index: None, // spoločný priestor
                    reporter_user_index: 9,  // najomnik1
                    assigned_user_index: Some(2), // technik
                    resolved_by_user_index: None,
                    confirmed_by_user_index: None,
                    rating: None,
                },
                // Porucha 2: Nová porucha elektroinštalácie v byte 2B
                SeedFault {
                    title: "Iskrenie elektrickej zásuvky v spálni",
                    description: "Pri dvojitej zásuvke na východnej stene spálne dochádza k iskreniu pri zapojení spotrebičov. Situácia je potenciálne nebezpečná.",
                    location_description: Some("Spálňa, východná stena"),
                    category: "electrical",
                    priority: "high",
                    status: "new",
                    building_index: 0,
                    unit_index: Some(3), // 2B
                    reporter_user_index: 9,  // najomnik1
                    assigned_user_index: None,
                    resolved_by_user_index: None,
                    confirmed_by_user_index: None,
                    rating: None,
                },
                // Porucha 3: Triedená porucha osvetlenia v Dubovej rezidencii
                SeedFault {
                    title: "Nefungujúce osvetlenie na 2. poschodí chodby",
                    description: "Tri svietidlá na chodbách 2. poschodia nefungujú. Priestor je tmavý, čo predstavuje bezpečnostné riziko v noci.",
                    location_description: Some("Chodba 2. poschodia"),
                    category: "electrical",
                    priority: "medium",
                    status: "triaged",
                    building_index: 1,
                    unit_index: None, // spoločný priestor
                    reporter_user_index: 5,  // kovacova
                    assigned_user_index: Some(2), // technik
                    resolved_by_user_index: None,
                    confirmed_by_user_index: None,
                    rating: None,
                },
                // Porucha 4: Vyriešená porucha okna v byte 201
                SeedFault {
                    title: "Poškodené tesnenie okna — prievan",
                    description: "Gumové tesnenie okna v obývačke je prasknuté a je cítiť výrazný prievan. Náklady na vykurovanie sa zvýšili.",
                    location_description: Some("Okno obývačky, južná strana"),
                    category: "structural",
                    priority: "low",
                    status: "resolved",
                    building_index: 1,
                    unit_index: Some(2), // 201
                    reporter_user_index: 10, // najomnik2
                    assigned_user_index: Some(2), // technik
                    resolved_by_user_index: Some(2), // technik
                    confirmed_by_user_index: None,
                    rating: None,
                },
                // Porucha 5: Nová porucha klimatizácie v Centrálnej plaze G1
                SeedFault {
                    title: "Klimatizácia nechladí komerčný priestor",
                    description: "Klimatizačná jednotka prestala chladiť komerčný priestor. Teplota vo vnútri dosiahla 32 °C, čo priestor robí nepoužiteľným.",
                    location_description: Some("Stropná klimatizačná jednotka"),
                    category: "heating",
                    priority: "medium",
                    status: "new",
                    building_index: 2,
                    unit_index: Some(0), // G1
                    reporter_user_index: 11, // najomnik3
                    assigned_user_index: None,
                    resolved_by_user_index: None,
                    confirmed_by_user_index: None,
                    rating: None,
                },
            ],
            votes: vec![
                // Hlasovanie 0: Uzavreté — pravidlá parkovania
                SeedVote {
                    title: "Pravidlá prideľovania parkovacích miest",
                    description: Some("Návrh na zavedenie číslovaných a vyhradených parkovacích miest pre každú bytovú jednotku s cieľom predísť konfliktom."),
                    building_index: 0,
                    status: "closed",
                    created_by_user_index: 1, // manazer
                    quorum_type: "simple_majority",
                    questions: vec![
                        SeedVoteQuestion {
                            question_text: "Súhlasíte s trvalým pridelením číslovaných parkovacích miest k jednotlivým bytovým jednotkám?",
                            question_type: "yes_no",
                            options: &[],
                        },
                    ],
                },
                // Hlasovanie 1: Aktívne — rozpočet na rekonštrukciu
                SeedVote {
                    title: "Rozpočet na rekonštrukciu budovy 2025",
                    description: Some("Schválenie rozpočtu na opravu fasády a úpravu spoločných priestorov. Celkové odhadované náklady sú 45 000 €, financované z rezervného fondu budovy."),
                    building_index: 0,
                    status: "active",
                    created_by_user_index: 1, // manazer
                    quorum_type: "two_thirds",
                    questions: vec![
                        SeedVoteQuestion {
                            question_text: "Súhlasíte so schválením rozpočtu 45 000 € na rekonštrukciu fasády a spoločných priestorov v roku 2025?",
                            question_type: "yes_no",
                            options: &[],
                        },
                        SeedVoteQuestion {
                            question_text: "Ktorá oblasť by mala mať pri rekonštrukcii prioritu?",
                            question_type: "single_choice",
                            options: &["Rekonštrukcia fasády", "Oprava strechy", "Úprava spoločných priestorov"],
                        },
                    ],
                },
            ],
            announcements: vec![
                // Oznámenie 0: Uvítacie, pripnuté, pre všetkých
                SeedAnnouncement {
                    title: "Vitajte v Demo správe nehnuteľností",
                    content: "# Vitajte\n\nVítame vás na portáli **Demo správa nehnuteľností**. Táto platforma je vaším centrálnym miestom pre správu vašej nehnuteľnosti, hlásenie porúch, účasť na hlasovaniach a prijímanie dôležitých oznámení.\n\nProsíme vás, aby ste si aktualizovali kontaktné údaje a povolili notifikácie, aby vám nič dôležité neušlo.\n\n*Tím správy*",
                    target_type: "all",
                    building_index: None,
                    status: "published",
                    pinned: true,
                    acknowledgment_required: false,
                    author_user_index: 0, // spravca
                },
                // Oznámenie 1: Prerušenie vody — len Bytový dom Hlavná
                SeedAnnouncement {
                    title: "Plánované prerušenie dodávky vody — Bytový dom Hlavná",
                    content: "## Oznámenie o prerušení vody\n\nVážení obyvatelia,\n\nUpozorňujeme vás na **plánované prerušenie dodávky vody** dňa **15. apríla od 08:00 do 14:00** z dôvodu údržby vodoinštalácie budovy.\n\nOdporúčame:\n- Vopred si naplniť zásobnú nádobu vodou\n- Vyhnúť sa praniu a umývaniu v tomto čase\n\nOspravedlňujeme sa za vzniknuté nepríjemnosti.\n\n*Správa budovy*",
                    target_type: "building",
                    building_index: Some(0), // Bytový dom Hlavná
                    status: "published",
                    pinned: false,
                    acknowledgment_required: false,
                    author_user_index: 1, // manazer
                },
                // Oznámenie 2: Výročné valné zhromaždenie — pre všetkých, potvrdenie účasti
                SeedAnnouncement {
                    title: "Výročné valné zhromaždenie — 20. mája 2025",
                    content: "## Výročné valné zhromaždenie\n\nPozývame vás na **Výročné valné zhromaždenie** Demo správy nehnuteľností.\n\n**Dátum:** Utorok 20. mája 2025  \n**Čas:** 18:00  \n**Miesto:** Zasadačka na prízemí, Bytový dom Hlavná\n\n### Program\n1. Správa o hospodárení za rok 2024\n2. Schválenie rozpočtu na údržbu 2025\n3. Voľba členov domovej rady\n4. Rôzne\n\nPotvrdenie prijatia tohto oznámenia je povinné. Zastúpenie formou splnomocnenia je povolené.\n\n*Správa budovy*",
                    target_type: "all",
                    building_index: None,
                    status: "published",
                    pinned: false,
                    acknowledgment_required: true,
                    author_user_index: 1, // manazer
                },
                // Oznámenie 3: Vianočné sviatky — archivované
                SeedAnnouncement {
                    title: "Vianočné sviatky 2024 — Obmedzená prevádzka",
                    content: "## Sviatočné oznámenie\n\nKancelária správy bude počas vianočných sviatkov (23. december – 2. január) fungovať v **obmedzenom režime**.\n\nHlásenia havarijných porúch sú možné cez tento portál kedykoľvek. Bežné požiadavky budú vybavené od **3. januára**.\n\nPrajeme všetkým obyvateľom príjemné a bezpečné sviatky.\n\n*Tím správy*",
                    target_type: "all",
                    building_index: None,
                    status: "archived",
                    pinned: false,
                    acknowledgment_required: false,
                    author_user_index: 0, // spravca
                },
            ],
            listings: vec![
                // Inzerát 0: 2-izbový byt na predaj — Hlavná 123, byt 1A
                SeedListing {
                    title: "2-izbový byt na predaj, Bratislava — Staré Mesto",
                    description: "Ponúkame na predaj svetlý 2-izbový byt v Bytovom dome Hlavná na Bratislavskom Starom Meste. Byt sa nachádza na 1. poschodí a má celkovú plochu 65 m². Nová kuchyňa s vybavením, svetlá obývacia izba s výhľadom na záhradu. Budova po kompletnej rekonštrukcii v roku 2015. Tichá lokalita s výbornou dopravnou dostupnosťou. Cena dohodou.",
                    transaction_type: "sale",
                    property_type: "apartment",
                    status: "active",
                    building_index: 0,
                    unit_index: Some(0), // 1A
                    created_by_user_index: 3, // novakova
                    price: 185_000,
                    is_negotiable: true,
                    rooms: Some(2),
                    bathrooms: Some(1),
                    features: &["Nová kuchyňa", "Parkovanie v suteréne", "Pivnica", "Výťah", "Plastové okná"],
                },
                // Inzerát 1: 3-izbový byt na prenájom — Hlavná 123, byt 2B
                SeedListing {
                    title: "3-izbový byt na prenájom, Bratislava — Hlavná ulica",
                    description: "Prenajmeme pekný 3-izbový byt na 2. poschodí Bytového domu Hlavná. Plocha 75 m², priestranná obývačka s jedálňou, kuchyňa, dve samostatné izby, kúpeľňa. Loggia s výhľadom na ulicu. Nájomné 950 €/mes, energetický paušál 150 €/mes. Preferovaný dlhodobý nájom minimálne 1 rok.",
                    transaction_type: "rent",
                    property_type: "apartment",
                    status: "active",
                    building_index: 0,
                    unit_index: Some(3), // 2B
                    created_by_user_index: 4, // horvath
                    price: 950,
                    is_negotiable: false,
                    rooms: Some(3),
                    bathrooms: Some(1),
                    features: &["Loggia", "Pivnica", "Výťah", "Blízkosť MHD", "Zariadený"],
                },
                // Inzerát 2: 2-izbový byt na predaj — Dubová 45, byt 101
                SeedListing {
                    title: "2-izbový byt na predaj, Bratislava — Dubová ulica",
                    description: "Predáme 2-izbový byt v modernej Dubovej rezidencii postavenej v roku 2015. Byt je na 1. poschodí s rozlohou 55 m². Kompletná rekonštrukcia, nové rozvody, výmenník tepla. Tichá ulica obklopená zeleňou, dostatok parkovacích miest v okolí. Výborná dopravná dostupnosť.",
                    transaction_type: "sale",
                    property_type: "apartment",
                    status: "active",
                    building_index: 1,
                    unit_index: Some(0), // 101
                    created_by_user_index: 8, // makler
                    price: 162_000,
                    is_negotiable: true,
                    rooms: Some(2),
                    bathrooms: Some(1),
                    features: &["Rekonštruovaný", "Tichá lokalita", "Parkovanie", "Zelené okolie"],
                },
                // Inzerát 3: 3-izbový byt na prenájom — Dubová 45, byt 201
                SeedListing {
                    title: "3-izbový byt na prenájom, Bratislava — Dubová rezidencia",
                    description: "Prenajmeme 3-izbový byt v Dubovej rezidencii na 2. poschodí. Plocha 70 m², obývačka s kuchynským kútom, dve spálne, kúpeľňa. Budova z roku 2015 v absolútne novom stave. Nájomné 800 €/mes vrátane všetkých poplatkov za správu. Nástup ihneď.",
                    transaction_type: "rent",
                    property_type: "apartment",
                    status: "active",
                    building_index: 1,
                    unit_index: Some(2), // 201
                    created_by_user_index: 8, // makler
                    price: 800,
                    is_negotiable: false,
                    rooms: Some(3),
                    bathrooms: Some(1),
                    features: &["Klimatizácia", "Rýchly internet", "Bezpečnostné dvere", "Zariadený"],
                },
                // Inzerát 4: 4-izbový byt na predaj — Centrálna 1, byt A1 (draft)
                SeedListing {
                    title: "4-izbový byt na predaj, Bratislava — Centrálna plaza",
                    description: "Exkluzívna ponuka 4-izbového bytu v prémiom projekte Centrálna plaza. Plocha 100 m², moderná otvorená kuchyňa, klimatizácia, podlahové kúrenie. Výhľad na centrum mesta. Byt v prvotriednom stave, kolaudácia 2020. Podzemná garáž v cene.",
                    transaction_type: "sale",
                    property_type: "apartment",
                    status: "draft",
                    building_index: 2,
                    unit_index: Some(2), // A1
                    created_by_user_index: 1, // manazer
                    price: 299_000,
                    is_negotiable: false,
                    rooms: Some(4),
                    bathrooms: Some(2),
                    features: &["Podlahové kúrenie", "Klimatizácia", "Prémiové vybavenie", "Garáž v cene", "Výhľad na centrum"],
                },
                // Inzerát 5: 2-izbový byt na prenájom — Hlavná 123, byt 2A
                SeedListing {
                    title: "2-izbový byt na prenájom, Bratislava — Staré Mesto",
                    description: "Prenajmeme 2-izbový byt na 2. poschodí Bytového domu Hlavná. Plocha 65 m², kuchyňa, obývačka, izba, kúpeľňa. Pôvodný stav, prenájom vhodný pre jednotlivca alebo pár. Nájomné 780 €/mes. Dostupné ihneď.",
                    transaction_type: "rent",
                    property_type: "apartment",
                    status: "active",
                    building_index: 0,
                    unit_index: Some(2), // 2A
                    created_by_user_index: 1, // manazer
                    price: 780,
                    is_negotiable: true,
                    rooms: Some(2),
                    bathrooms: Some(1),
                    features: &["Výťah", "Pivnica", "MHD dostupnosť", "Centrum mesta"],
                },
            ],
            portal_users: vec![
                // Portálový používateľ 0: Záujemkyňa o kúpu bytu
                SeedPortalUser {
                    email: "zuzana@demo-reality.test",
                    name: "Zuzana Procházková",
                    phone: Some("+421911100200"),
                    pm_user_index: None,
                },
                // Portálový používateľ 1: Záujemca o prenájom
                SeedPortalUser {
                    email: "rastislav@demo-reality.test",
                    name: "Rastislav Sloboda",
                    phone: Some("+421911100201"),
                    pm_user_index: None,
                },
                // Portálový používateľ 2: Záujemkyňa o kúpu
                SeedPortalUser {
                    email: "katarina@demo-reality.test",
                    name: "Katarína Hrušková",
                    phone: Some("+421911100202"),
                    pm_user_index: None,
                },
                // Portálový používateľ 3: Realitný maklér — prepojený na PPT makléra
                SeedPortalUser {
                    email: "makler@demo-reality.test",
                    name: "Tomáš Maklér",
                    phone: Some("+421900333002"),
                    pm_user_index: Some(8), // makler (PPT user index 8)
                },
            ],
            inquiries: vec![
                // Dopyt 0: Zuzana žiada prehliadku bytu 1A (predaj)
                SeedInquiry {
                    listing_index: 0, // 2-izbový byt, predaj
                    portal_user_index: 0, // zuzana
                    realtor_portal_user_index: 3, // makler
                    message: "Dobrý deň, zaujíma ma prehliadka tohto bytu. Kedy by bolo možné si ho pozrieť? K dispozícii som pracovné dni od 16:00, prípadne cez víkend. S pozdravom, Zuzana Procházková",
                    inquiry_type: "viewing",
                    preferred_contact: "email",
                    status: "responded",
                    reply_message: Some("Dobrý deň, pani Procházková, radi vám byt ukážeme. Môžeme sa stretnúť v piatok 18. apríla o 17:00. Potvrďte prosím vašu účasť odpoveďou na tento email. S pozdravom, Tomáš Maklér"),
                },
                // Dopyt 1: Rastislav sa pýta na podmienky prenájmu bytu 2B
                SeedInquiry {
                    listing_index: 1, // 3-izbový byt, prenájom
                    portal_user_index: 1, // rastislav
                    realtor_portal_user_index: 3, // makler
                    message: "Dobrý deň, mám záujem o tento byt na prenájom. Aká je minimálna dĺžka nájomnej zmluvy? Sú povolené domáce zvieratá (malý pes)? Ďakujem za informácie. Rastislav Sloboda",
                    inquiry_type: "info",
                    preferred_contact: "email",
                    status: "read",
                    reply_message: None,
                },
                // Dopyt 2: Katarína robí ponuku na byt 101
                SeedInquiry {
                    listing_index: 2, // 2-izbový byt, predaj Dubová
                    portal_user_index: 2, // katarina
                    realtor_portal_user_index: 3, // makler
                    message: "Dobrý deň, po osobnej obhliadke bytu by som rada urobila ponuku vo výške 155 000 €. Byt ma veľmi zaujal a som pripravená podpísať rezervačnú zmluvu tento týždeň. Môžete mi potvrdiť záujem predávajúceho? Ďakujem. Katarína Hrušková",
                    inquiry_type: "offer",
                    preferred_contact: "phone",
                    status: "new",
                    reply_message: None,
                },
            ],
            portal_favorites: vec![
                // Obľúbené 0: Zuzana si uložila byt 101 na Dubovej
                SeedPortalFavorite {
                    portal_user_index: 0, // zuzana
                    listing_index: 2,    // byt 101, predaj
                },
                // Obľúbené 1: Rastislav si uložil prenájom na Dubovej
                SeedPortalFavorite {
                    portal_user_index: 1, // rastislav
                    listing_index: 3,    // byt 201, prenájom
                },
                // Obľúbené 2: Katarína si uložila byt 1A na predaj
                SeedPortalFavorite {
                    portal_user_index: 2, // katarina
                    listing_index: 0,    // byt 1A, predaj
                },
            ],
            default_password: "DemoHeslo123",
            portal_default_password: "PortalHeslo123",
        }
    }
}

impl SeedData {
    /// Vytvorí nové seed dáta s vlastnou konfiguráciou organizácie.
    pub fn new(organization: SeedOrganization) -> Self {
        Self {
            organization,
            buildings: Vec::new(),
            users: Vec::new(),
            faults: Vec::new(),
            votes: Vec::new(),
            announcements: Vec::new(),
            listings: Vec::new(),
            portal_users: Vec::new(),
            inquiries: Vec::new(),
            portal_favorites: Vec::new(),
            default_password: "DemoHeslo123",
            portal_default_password: "PortalHeslo123",
        }
    }

    /// Vráti celkový počet bytových jednotiek naprieč všetkými budovami.
    pub fn total_units(&self) -> usize {
        self.buildings.iter().map(|b| b.units.len()).sum()
    }
}
