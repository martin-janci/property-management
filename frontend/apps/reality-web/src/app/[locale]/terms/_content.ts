/**
 * Terms of Service — Slovak source. NOT LEGALLY REVIEWED. This is realistic
 * placeholder copy for the design-integration phase; before production launch,
 * the entire ToS must be replaced with text reviewed by legal counsel.
 *
 * Effective date should be updated whenever sections change.
 */

export const TERMS_META = {
  effective: '1. január 2026',
  lastUpdated: '10. máj 2026',
};

export interface TermsSection {
  id: string;
  heading: string;
  paragraphs?: string[];
  bullets?: string[];
}

export const TERMS_SECTIONS: TermsSection[] = [
  {
    id: 'acceptance',
    heading: '1. Súhlas s podmienkami',
    paragraphs: [
      'Tieto Podmienky používania (ďalej len "Podmienky") upravujú váš prístup a používanie webovej stránky Reality Portál (ďalej len "Služba") prevádzkovanej spoločnosťou Reality Portál s.r.o., so sídlom Mlynské nivy 5, 821 09 Bratislava (ďalej len "Prevádzkovateľ").',
      'Vstupom na Službu alebo jej používaním vyjadrujete svoj súhlas s týmito Podmienkami. Ak s nimi nesúhlasíte, Službu nepoužívajte.',
    ],
  },
  {
    id: 'definitions',
    heading: '2. Definície',
    bullets: [
      '"Používateľ" — fyzická alebo právnická osoba, ktorá navštevuje alebo využíva Službu, či už registrovaná alebo neregistrovaná.',
      '"Inzerát" — ponuka nehnuteľnosti zverejnená Predávajúcim na Službe.',
      '"Predávajúci" — Používateľ, ktorý zverejňuje Inzerát (súkromná osoba, realitný maklér alebo agentúra).',
      '"Záujemca" — Používateľ, ktorý prejaví záujem o Inzerát prostredníctvom Služby.',
      '"Obsah" — texty, fotografie, video a ďalšie materiály zverejnené na Službe.',
    ],
  },
  {
    id: 'account',
    heading: '3. Registrácia a účet',
    paragraphs: [
      'Niektoré funkcie Služby vyžadujú vytvorenie používateľského účtu. Pri registrácii ste povinní poskytnúť pravdivé a aktuálne údaje a tieto údaje udržiavať aktuálne.',
      'Za bezpečnosť svojho hesla a všetky aktivity vykonávané pod vaším účtom zodpovedáte výhradne vy. Akékoľvek podozrenie zo zneužitia účtu nahláste bez zbytočného odkladu na security@rlt.sk.',
      'Prevádzkovateľ si vyhradzuje právo zrušiť účet, ktorý porušuje tieto Podmienky alebo bol vytvorený s neplatnými údajmi.',
    ],
  },
  {
    id: 'listings',
    heading: '4. Zverejňovanie inzerátov',
    paragraphs: [
      'Predávajúci je povinný zverejňovať pravdivé, úplné a aktuálne informácie o nehnuteľnosti. Fotografie musia zodpovedať skutočnému stavu nehnuteľnosti a Predávajúci musí mať právo s nimi disponovať.',
      'Zakázané je najmä zverejňovať Inzeráty obsahujúce zavádzajúce informácie, neexistujúce nehnuteľnosti, duplicitné Inzeráty pre tú istú nehnuteľnosť alebo Inzeráty propagujúce protizákonné aktivity.',
    ],
    bullets: [
      'Súkromní predávajúci — Inzeráty zadarmo bez časového obmedzenia, max. 3 aktívne Inzeráty súčasne.',
      'Realitní makléri — podľa zvoleného balíka, vrátane priorizovaného zobrazenia a hromadného importu.',
      'Agentúry — vlastný profil, integrácia s CRM cez API, fakturačné podmienky podľa zmluvy.',
    ],
  },
  {
    id: 'fees',
    heading: '5. Cena a platby',
    paragraphs: [
      'Inzeráty pre súkromných Predávajúcich sú bezplatné. Profesionálne balíky pre maklérov a agentúry sú spoplatnené podľa aktuálneho cenníka zverejneného na stránke /for-agents.',
      'Platby sa vykonávajú prostredníctvom platobnej brány zazmluvneného poskytovateľa. Faktúry sú vystavené elektronicky a doručené na e-mail uvedený pri registrácii.',
      'Pri predčasnom ukončení predplatného sa nevyplatená pomerná časť nevracia, pokiaľ neuvádza zákon inak.',
    ],
  },
  {
    id: 'liability',
    heading: '6. Zodpovednosť',
    paragraphs: [
      'Prevádzkovateľ je výlučne sprostredkovateľom medzi Predávajúcim a Záujemcom. Nezodpovedá za pravdivosť obsahu Inzerátov, kvalitu nehnuteľnosti, ani za priebeh akejkoľvek dohody medzi Predávajúcim a Záujemcom.',
      'Prevádzkovateľ vyvíja primerané úsilie na zabezpečenie dostupnosti Služby, neručí však za nepretržitú prevádzku ani za stratu alebo poškodenie údajov v dôsledku okolností mimo jeho kontroly.',
      'Maximálna výška náhrady škody, ktorú môže Prevádzkovateľ uhradiť Používateľovi, je obmedzená na sumu, ktorú Používateľ uhradil za Služby v posledných 12 mesiacoch.',
    ],
  },
  {
    id: 'ip',
    heading: '7. Duševné vlastníctvo',
    paragraphs: [
      'Všetky práva k Službe, vrátane jej dizajnu, kódu, ochrannej známky a dokumentácie, patria Prevádzkovateľovi alebo jeho licenčným partnerom.',
      'Zverejnením Inzerátu udeľujete Prevádzkovateľovi nevýhradnú, bezplatnú licenciu na používanie poskytnutého Obsahu na účely zobrazenia, propagácie a indexovania v rozsahu nevyhnutnom pre prevádzku Služby.',
    ],
  },
  {
    id: 'termination',
    heading: '8. Ukončenie',
    paragraphs: [
      'Používateľ môže kedykoľvek zrušiť svoj účet v sekcii Nastavenia. Po zrušení účtu sú aktívne Inzeráty stiahnuté do 24 hodín; archivované údaje uchovávame v rozsahu povinnom zákonom (faktúry, daňové doklady).',
      'Prevádzkovateľ môže ukončiť účet alebo obmedziť prístup k Službe pri závažnom alebo opakovanom porušení Podmienok, vrátane podvodných Inzerátov, zneužitia identity alebo neoprávneného prístupu k iným účtom.',
    ],
  },
  {
    id: 'changes',
    heading: '9. Zmeny Podmienok',
    paragraphs: [
      'Prevádzkovateľ si vyhradzuje právo tieto Podmienky kedykoľvek zmeniť. O materiálnych zmenách budeme informovať zaregistrovaných Používateľov e-mailom najmenej 14 dní pred ich účinnosťou. Pokračovaním v používaní Služby po dátume účinnosti potvrdzujete súhlas so zmenenými Podmienkami.',
    ],
  },
  {
    id: 'jurisdiction',
    heading: '10. Rozhodné právo a riešenie sporov',
    paragraphs: [
      'Tieto Podmienky a vzťahy z nich vyplývajúce sa riadia právnym poriadkom Slovenskej republiky. Príslušným súdom pre riešenie sporov je všeobecný súd Prevádzkovateľa, pokiaľ kogentné ustanovenia právneho poriadku neurčia inak (napr. spotrebiteľské spory).',
      'Spotrebitelia majú právo obrátiť sa na orgán alternatívneho riešenia sporov v zmysle zákona č. 391/2015 Z. z., konkrétne na Slovenskú obchodnú inšpekciu (www.soi.sk).',
    ],
  },
  {
    id: 'contact',
    heading: '11. Kontakt',
    paragraphs: [
      'Otázky k Podmienkam, námietky alebo žiadosti zasielajte na info@rlt.sk alebo poštou na sídlo spoločnosti uvedené vyššie.',
    ],
  },
];
