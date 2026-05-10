export interface CookieRow {
  name: string;
  provider: string;
  purpose: string;
  duration: string;
  type: 'essential' | 'functional' | 'analytics' | 'marketing';
}

export const MOCK_COOKIES: CookieRow[] = [
  {
    name: 'ppt_session',
    provider: 'rlt.sk',
    purpose: 'Udržuje prihlásenie používateľa',
    duration: '7 dní',
    type: 'essential',
  },
  {
    name: 'ppt_locale',
    provider: 'rlt.sk',
    purpose: 'Uloží preferovaný jazyk',
    duration: '1 rok',
    type: 'functional',
  },
  {
    name: 'ppt_cs',
    provider: 'rlt.sk',
    purpose: 'Uloží voľby súhlasu cookies',
    duration: '1 rok',
    type: 'essential',
  },
  {
    name: '_ga',
    provider: 'google.com',
    purpose: 'Google Analytics – rozlíšenie návštevníkov',
    duration: '2 roky',
    type: 'analytics',
  },
  {
    name: '_ga_*',
    provider: 'google.com',
    purpose: 'Google Analytics – dĺžka relácie',
    duration: '2 roky',
    type: 'analytics',
  },
  {
    name: 'fbp',
    provider: 'facebook.com',
    purpose: 'Facebook Pixel – sledovanie konverzií',
    duration: '3 mesiace',
    type: 'marketing',
  },
  {
    name: 'intercom-id-*',
    provider: 'intercom.com',
    purpose: 'Živý chat a podpora',
    duration: '9 mesiacov',
    type: 'functional',
  },
];

export const COOKIE_SECTIONS = [
  {
    id: 'what',
    heading: '§ 1 Čo sú cookies',
    body: 'Cookies sú malé textové súbory ukladané vo vašom prehliadači, keď navštívite webovú stránku. Umožňujú stránke zapamätať si vaše nastavenia a zlepšiť váš zážitok.',
  },
  {
    id: 'types',
    heading: '§ 2 Typy cookies, ktoré používame',
    body: 'Používame nevyhnutné cookies pre správne fungovanie stránky, funkčné cookies pre personalizáciu, analytické cookies na zlepšenie služieb a marketingové cookies na relevantné reklamy.',
  },
  {
    id: 'control',
    heading: '§ 3 Vaša kontrola',
    body: 'Cookies môžete spravovať v nastaveniach svojho prehliadača. Vypnutie niektorých cookies môže obmedziť funkčnosť webu.',
  },
  {
    id: 'third-party',
    heading: '§ 4 Cookies tretích strán',
    body: 'Niektoré cookies pochádzajú od tretích strán (napr. Google, Facebook). Na tieto cookies sa vzťahujú ich vlastné zásady ochrany súkromia.',
  },
  {
    id: 'updates',
    heading: '§ 5 Aktualizácie tejto politiky',
    body: 'Túto politiku môžeme pravidelne aktualizovať. Odporúčame ju pravidelne kontrolovať. Dátum poslednej aktualizácie: 1. januára 2025.',
  },
];
