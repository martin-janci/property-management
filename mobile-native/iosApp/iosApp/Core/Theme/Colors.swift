import SwiftUI

// MARK: - Brand

extension Color {
    static let brand50  = Color(hex: 0xEFF6FF)
    static let brand100 = Color(hex: 0xDBEAFE)
    static let brand200 = Color(hex: 0xBFDBFE)
    static let brand300 = Color(hex: 0x93C5FD)
    static let brand400 = Color(hex: 0x60A5FA)
    static let brand500 = Color(hex: 0x3B82F6)
    static let brand600 = Color(hex: 0x2563EB) // PRIMARY
    static let brand700 = Color(hex: 0x1D4ED8) // primary-hover
    static let brand800 = Color(hex: 0x1E40AF) // hero gradient start
    static let brand900 = Color(hex: 0x1E3A8A)
}

// MARK: - Semantic

extension Color {
    static let pptPrimary      = Color.brand600
    static let pptPrimaryHover = Color.brand700

    static let pptDanger       = Color(hex: 0xEF4444)
    static let pptDangerHover  = Color(hex: 0xDC2626)
    static let pptDangerLight  = Color(hex: 0xFEE2E2)
    static let pptDangerDark   = Color(hex: 0x991B1B)

    static let pptWarning      = Color(hex: 0xF59E0B)
    static let pptWarningHover = Color(hex: 0xD97706)
    static let pptWarningLight = Color(hex: 0xFEF3C7)
    static let pptWarningDark  = Color(hex: 0x92400E)

    static let pptSuccess      = Color(hex: 0x10B981)
    static let pptSuccessLight = Color(hex: 0xD1FAE5)
    static let pptSuccessDark  = Color(hex: 0x065F46)

    static let pptViolet       = Color(hex: 0x8B5CF6)
    static let pptVioletLight  = Color(hex: 0xEDE9FE)
    static let pptVioletDark   = Color(hex: 0x5B21B6)

    static let pptOrange       = Color(hex: 0xF97316)
    static let pptOrangeLight  = Color(hex: 0xFFEDD5)
    static let pptOrangeDark   = Color(hex: 0x9A3412)
}

// MARK: - Fault status pill token pairs

struct FaultStatusPalette {
    let bg: Color
    let ink: Color
}

enum FaultStatusColors {
    static let new        = FaultStatusPalette(bg: Color(hex: 0xFEE2E2), ink: Color(hex: 0x991B1B))
    static let triaged    = FaultStatusPalette(bg: Color(hex: 0xDBEAFE), ink: Color(hex: 0x1E40AF))
    static let inProgress = FaultStatusPalette(bg: Color(hex: 0xFEF3C7), ink: Color(hex: 0x92400E))
    static let waitParts  = FaultStatusPalette(bg: Color(hex: 0xFFEDD5), ink: Color(hex: 0x9A3412))
    static let scheduled  = FaultStatusPalette(bg: Color(hex: 0xEDE9FE), ink: Color(hex: 0x5B21B6))
    static let resolved   = FaultStatusPalette(bg: Color(hex: 0xD1FAE5), ink: Color(hex: 0x065F46))
    static let closed     = FaultStatusPalette(bg: Color(hex: 0xF3F4F6), ink: Color(hex: 0x374151))
    static let reopened   = FaultStatusPalette(bg: Color(hex: 0xFEE2E2), ink: Color(hex: 0x991B1B))
}

// MARK: - Priority ink colors

enum PriorityColors {
    static let low    = Color(hex: 0x6B7280)
    static let medium = Color(hex: 0xD97706)
    static let high   = Color(hex: 0xF97316)
    static let urgent = Color(hex: 0xDC2626)
}

// MARK: - Listing / reality-portal badge colors

struct BadgePalette {
    let bg: Color
    let ink: Color
}

enum BadgeColors {
    static let sale     = BadgePalette(bg: Color(hex: 0x10B981), ink: .white)
    static let rent     = BadgePalette(bg: Color(hex: 0x3B82F6), ink: .white)
    static let featured = BadgePalette(bg: Color(hex: 0xFBBF24), ink: Color(hex: 0x78350F))
}

// MARK: - Inquiry status colors (reality-portal)

struct InquiryStatusPalette {
    let bg: Color
    let ink: Color
}

enum InquiryStatusColors {
    static let pending  = InquiryStatusPalette(bg: Color.pptWarningLight, ink: Color.pptWarning)
    static let replied  = InquiryStatusPalette(bg: Color.pptSuccessLight, ink: Color.pptSuccessDark)
    static let closed   = InquiryStatusPalette(bg: Color(hex: 0xF3F4F6),  ink: Color(hex: 0x374151))
}

// MARK: - Hex initializer

private extension Color {
    init(hex: UInt32) {
        self.init(
            red:   Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >>  8) & 0xFF) / 255,
            blue:  Double( hex        & 0xFF) / 255
        )
    }
}
