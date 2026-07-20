import SwiftUI

/// Placeholder block rendered in place of a layout section that is present in
/// the resolved layout but has `presentation == "placeholder"`.
///
/// Styled to match the app's other empty/placeholder states (secondary colour,
/// rounded rect, ~96pt minimum height).
struct LayoutPlaceholderSection: View {
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 12)
                .fill(Color(.systemGray6))
                .frame(minHeight: 96)

            Text(String(localized: "section_unavailable"))
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
    }
}
