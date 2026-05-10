/**
 * Privacy Policy — Slovak source. NOT LEGALLY REVIEWED. Realistic placeholder
 * copy aligned with GDPR (Nariadenie 2016/679) and zákonom o ochrane osobných
 * údajov (18/2018 Z. z.). Replace with text reviewed by DPO before production.
 */

export const PRIVACY_META = {
  effective: '1. január 2026',
  lastUpdated: '10. máj 2026',
  controller: 'Reality Portál s.r.o., Mlynské nivy 5, 821 09 Bratislava',
};

export interface PrivacySection {
  id: string;
  heading: string;
  paragraphs?: string[];
  bullets?: string[];
}

export const PRIVACY_SECTIONS: PrivacySection[] = [
  {
    id: 'controller',
    heading: '1. Kto spracúva vaše údaje',
    paragraphs: [
      `Prevádzkovateľom v zmysle čl. 4 GDPR je ${PRIVACY_META.controller}.`,
      'Pre otázky týkajúce sa ochrany osobných údajov nás kontaktujte na privacy@rlt.sk. Zodpovednú osobu (DPO) môžete kontaktovať na rovnakej adrese.',
    ],
  },
  {
    id: 'data',
    heading: '2. Aké údaje spracúvame',
    bullets: [
      'Identifikačné — meno, e-mail, telefónne číslo (pri registrácii a v inzerátoch).',
      'Lokalizačné — adresa nehnuteľnosti pri Inzeráte; pri vyhľadávaní približná lokalita zariadenia (s vaším súhlasom).',
      'Komunikačné — obsah správ medzi Záujemcami a Predávajúcimi cez náš portál.',
      'Technické — IP adresa, identifikátor zariadenia, údaje o prehliadači, log súborov.',
      'Behaviorálne — záznamy prehliadania a interakcií so Službou pre účely zlepšovania UX.',
      'Platobné — pri profesionálnych účtoch fakturačné údaje a história transakcií (samotné údaje karty spracúva platobná brána, my ich nevidíme).',
    ],
  },
  {
    id: 'purposes',
    heading: '3. Na aký účel a na akom právnom základe',
    paragraphs: [
      'Vaše osobné údaje spracúvame na nasledovných právnych základoch v zmysle čl. 6 GDPR:',
    ],
    bullets: [
      'Plnenie zmluvy (čl. 6 ods. 1 písm. b) — zriadenie účtu, zverejnenie Inzerátu, sprostredkovanie kontaktu.',
      'Plnenie zákonných povinností (čl. 6 ods. 1 písm. c) — účtovníctvo, daňová evidencia, archivácia.',
      'Oprávnený záujem (čl. 6 ods. 1 písm. f) — bezpečnosť Služby, prevencia podvodov, priamy marketing existujúcim zákazníkom.',
      'Súhlas (čl. 6 ods. 1 písm. a) — analytické a marketingové cookies, newsletter pre neregistrovaných.',
    ],
  },
  {
    id: 'sharing',
    heading: '4. Komu vaše údaje sprístupňujeme',
    paragraphs: ['Údaje neposkytujeme tretím stranám okrem nasledovných prípadov:'],
    bullets: [
      'Sprostredkovatelia — poskytovatelia hostingu (Hetzner, Cloudflare), e-mailovej infraštruktúry a platobnej brány.',
      'Realitní makléri / agentúry — pri prejave záujmu o ich Inzerát.',
      'Štátne orgány — výlučne na základe oprávnenej žiadosti podľa zákona.',
      'Akvizícia / restrukturalizácia — v prípade transferu Služby na nástupníckeho prevádzkovateľa, ktorý prevezme rovnaké povinnosti.',
    ],
  },
  {
    id: 'transfers',
    heading: '5. Prenos údajov mimo EÚ',
    paragraphs: [
      'Vaše údaje primárne spracúvame v dátových centrách v Európskej únii (Nemecko, Holandsko). V určitých prípadoch (napr. pri použití analytických nástrojov) môže dôjsť k prenosu do tretích krajín; v takom prípade aplikujeme štandardné zmluvné doložky schválené Európskou komisiou alebo iné záruky podľa kapitoly V GDPR.',
    ],
  },
  {
    id: 'retention',
    heading: '6. Ako dlho údaje uchovávame',
    bullets: [
      'Aktívny účet — po celú dobu existencie účtu.',
      'Inzeráty — počas platnosti Inzerátu + 90 dní v archíve pre prípad sporu.',
      'Účtovné doklady — 10 rokov v zmysle daňových zákonov.',
      'Marketingové súhlasy — do odvolania súhlasu.',
      'Technické logy — 12 mesiacov pre účely bezpečnosti a analýzy incidentov.',
    ],
  },
  {
    id: 'rights',
    heading: '7. Vaše práva',
    paragraphs: ['V zmysle čl. 15–22 GDPR máte právo:'],
    bullets: [
      'Na prístup k údajom — vyžiadať si kópiu spracúvaných údajov.',
      'Na opravu — žiadať opravu nesprávnych alebo doplnenie neúplných údajov.',
      'Na vymazanie ("zabudnutie") — za podmienok stanovených zákonom.',
      'Na obmedzenie spracúvania — pozastaviť určité operácie pri sporných údajoch.',
      'Na prenosnosť — získať údaje v štruktúrovanom strojovo čitateľnom formáte.',
      'Namietať proti spracúvaniu — najmä proti priamemu marketingu.',
      'Odvolať súhlas — kedykoľvek, bez dopadu na zákonnosť spracúvania pred odvolaním.',
      'Podať sťažnosť na Úrade na ochranu osobných údajov SR (www.dataprotection.gov.sk).',
    ],
  },
  {
    id: 'cookies',
    heading: '8. Cookies a sledovacie technológie',
    paragraphs: [
      'Podrobné informácie o cookies a sledovacích technológiách nájdete v Zásadách používania cookies na /cookies. Tam môžete tiež kedykoľvek upraviť svoje preferencie.',
    ],
  },
  {
    id: 'security',
    heading: '9. Bezpečnosť údajov',
    paragraphs: [
      'Aplikujeme primerané technické a organizačné opatrenia na ochranu vašich údajov: šifrovanie pri prenose (TLS), šifrovanie v pokoji pri citlivých údajoch, viacúrovňové oprávnenia, pravidelné audity, segregáciu produkčných a vývojových prostredí.',
      'V prípade bezpečnostného incidentu, ktorý by mohol predstavovať vysoké riziko pre vaše práva, vás budeme informovať bez zbytočného odkladu, najneskôr však do 72 hodín od zistenia (čl. 33 GDPR).',
    ],
  },
  {
    id: 'minors',
    heading: '10. Osoby mladšie ako 16 rokov',
    paragraphs: [
      'Služba nie je určená osobám mladším ako 16 rokov. Vedome nezbierame údaje o maloletých. Ak sa dozvieme, že sme údaje takejto osoby získali, bezodkladne ich odstránime.',
    ],
  },
  {
    id: 'changes',
    heading: '11. Zmeny zásad',
    paragraphs: [
      'Tieto Zásady ochrany súkromia môžeme aktualizovať. O materiálnych zmenách informujeme registrovaných používateľov e-mailom najmenej 14 dní pred ich účinnosťou.',
    ],
  },
  {
    id: 'contact',
    heading: '12. Kontakt',
    paragraphs: [
      'Otázky a žiadosti o uplatnenie práv zasielajte na privacy@rlt.sk alebo na adresu sídla. Odpovedáme zvyčajne do 30 dní.',
    ],
  },
];
