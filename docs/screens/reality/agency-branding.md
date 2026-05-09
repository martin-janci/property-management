---
id: reality/agency-branding
name: Agency Branding
product: reality
implementations:
  reality-web:
    component: AgencyBrandingPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: reality/agency-dashboard
    rel: parent
  - id: reality/agency-import
    rel: sibling
sharedComponents:
  - tabs
  - file-upload
  - color-picker
  - text-input
  - chip-group
  - preview-pane
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/agency-branding.html
    frame: default-with-profile-logo-color-watermark / loading
useCases:
  - UC-49
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header
- [ ] [w] Manager chrome + agency-dashboard tab strip with `Branding` active
- [ ] [w] Page H1 "Branding agentúry"
- [ ] [w] Right: "Náhľad" ghost (opens preview drawer) + "Uložiť zmeny" primary

### Section · Profil agentúry
- [ ] [w] Agency name input · domain input · description textarea · primary contact email · phone-input · region chips
- [ ] [w] Live preview tile on right showing how this renders on listings

### Section · Logo
- [ ] [w] File-upload dropzone (PNG/SVG, transparent bg recommended) · alt-text input · size hint
- [ ] [w] Preview at 3 sizes (header, listing-card, mobile)

### Section · Hlavná farba
- [ ] [w] Color-picker per `forms/color-picker.html` (preset palette + custom HEX input)
- [ ] [w] Live preview of agency CTAs + badges in chosen color
- [ ] [w] Contrast check warning if chosen color fails WCAG AA against white

### Section · Štýl vodoznaku
- [ ] [w] 4 watermark style radio-cards: žiadny · v rohu · cez celú fotku · vlastný (upload PNG)
- [ ] [w] Opacity slider (10–60%)
- [ ] [w] Position chips (top-left / top-right / bottom-left / bottom-right / center)
- [ ] [w] Preview on a sample listing photo

### Save bar (sticky)
- [ ] [w] Idle / dirty (with "N changes") / saving / error states matching `ppt-accessibility-settings.html` save-bar pattern

## States

- **Default**: profile + logo + color + watermark filled, save bar idle
- **Loading**: section skeletons; tabs interactive
- **Saving / saved (toast)**: matches accessibility-settings save-bar
- **Error**: top banner + retry; local changes preserved

## Notes

### Broader context

UC-49 agency self-service branding. Customizes how the agency appears on listings (header logo, primary color in CTAs, watermark on photos). Live preview is the killer feature — see effect before save.

### Specific (recent)

- Watermark "vlastný" upload accepts PNG with transparency; max 2 MB; recommended 200×200 minimum.
- Contrast check uses WCAG 2.2 AA (4.5:1 for normal text); warn but allow override for design intent.
- Color-picker presets should include the brand palette + 6 popular agency colors (red, blue, green, gold, purple, charcoal).
- Photos with watermark are watermarked **at upload time** server-side; existing photos are NOT re-watermarked when settings change (avoid breaking older photos in cache).

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/agency-branding.html — default + loading); UC-49; parent agency-dashboard; sibling agency-import
