package three.two.bit.ppt.reality.util

import kotlin.test.Test
import kotlin.test.assertEquals
import three.two.bit.ppt.reality.listing.Address

class FormatUtilsTest {

    private fun address(
        street: String? = null,
        city: String = "Bratislava",
        district: String? = null,
        region: String? = null,
        postalCode: String? = null,
        country: String = "SK"
    ): Address = Address(
        street = street,
        city = city,
        district = district,
        region = region,
        postalCode = postalCode,
        country = country
    )

    // -------- formatPrice (currency symbol prefix) --------

    @Test
    fun formatPrice_eur_uses_euro_symbol_prefix() {
        assertEquals("\u20AC500", FormatUtils.formatPrice(500, "EUR"))
    }

    @Test
    fun formatPrice_usd_uses_dollar_prefix() {
        assertEquals("$500", FormatUtils.formatPrice(500, "USD"))
    }

    @Test
    fun formatPrice_gbp_uses_pound_prefix() {
        assertEquals("\u00A3500", FormatUtils.formatPrice(500, "GBP"))
    }

    @Test
    fun formatPrice_unknown_currency_falls_back_to_suffix_with_code() {
        assertEquals("500 CZK", FormatUtils.formatPrice(500, "CZK"))
        assertEquals("1,234 PLN", FormatUtils.formatPrice(1_234, "PLN"))
    }

    // -------- formatPriceAmount via formatPrice --------

    @Test
    fun formatPrice_below_one_thousand_has_no_separator() {
        assertEquals("$1", FormatUtils.formatPrice(1, "USD"))
        assertEquals("$999", FormatUtils.formatPrice(999, "USD"))
    }

    @Test
    fun formatPrice_thousands_inserts_commas() {
        assertEquals("$1,000", FormatUtils.formatPrice(1_000, "USD"))
        assertEquals("$12,345", FormatUtils.formatPrice(12_345, "USD"))
        assertEquals("$999,999", FormatUtils.formatPrice(999_999, "USD"))
    }

    @Test
    fun formatPrice_one_million_renders_as_1_00M() {
        assertEquals("$1.00M", FormatUtils.formatPrice(1_000_000, "USD"))
    }

    @Test
    fun formatPrice_millions_truncated_to_two_decimals() {
        // 1_234_567 / 10_000 = 123 -> integer 1, fractional 23 -> "1.23M"
        assertEquals("$1.23M", FormatUtils.formatPrice(1_234_567, "USD"))
    }

    @Test
    fun formatPrice_millions_pads_fractional_under_ten() {
        // 1_050_000 / 10_000 = 105 -> integer 1, fractional 05 -> "1.05M"
        assertEquals("$1.05M", FormatUtils.formatPrice(1_050_000, "USD"))
    }

    @Test
    fun formatPrice_millions_truncates_not_rounds() {
        // 1_999_999 / 10_000 = 199 -> "1.99M" (not 2.00M)
        assertEquals("$1.99M", FormatUtils.formatPrice(1_999_999, "USD"))
    }

    @Test
    fun formatPrice_zero_amount() {
        assertEquals("$0", FormatUtils.formatPrice(0, "USD"))
        assertEquals("0 CZK", FormatUtils.formatPrice(0, "CZK"))
    }

    @Test
    fun formatPrice_large_millions_value() {
        // 25_678_900 / 10_000 = 2567 -> integer 25, fractional 67 -> "25.67M"
        assertEquals("$25.67M", FormatUtils.formatPrice(25_678_900, "USD"))
    }

    // -------- buildLocationString --------

    @Test
    fun buildLocationString_default_excludes_street_and_postal() {
        val addr = address(street = "Hlavna 1", city = "Bratislava", district = "Old Town", postalCode = "81101", region = "BA")
        assertEquals("Old Town, Bratislava, BA", FormatUtils.buildLocationString(addr))
    }

    @Test
    fun buildLocationString_includes_street_when_requested() {
        val addr = address(street = "Hlavna 1", city = "Bratislava")
        assertEquals("Hlavna 1, Bratislava", FormatUtils.buildLocationString(addr, includeStreet = true))
    }

    @Test
    fun buildLocationString_includes_postal_when_requested() {
        val addr = address(city = "Bratislava", postalCode = "81101")
        assertEquals("81101, Bratislava", FormatUtils.buildLocationString(addr, includePostalCode = true))
    }

    @Test
    fun buildLocationString_includes_all_parts_when_requested() {
        val addr = address(street = "Hlavna 1", city = "Bratislava", district = "Old Town", region = "BA", postalCode = "81101")
        assertEquals(
            "Hlavna 1, 81101, Old Town, Bratislava, BA",
            FormatUtils.buildLocationString(addr, includeStreet = true, includePostalCode = true)
        )
    }

    @Test
    fun buildLocationString_skips_null_optional_parts() {
        val addr = address(city = "Bratislava")
        assertEquals("Bratislava", FormatUtils.buildLocationString(addr))
    }

    @Test
    fun buildLocationString_skips_street_when_null_even_if_requested() {
        val addr = address(city = "Bratislava")
        assertEquals("Bratislava", FormatUtils.buildLocationString(addr, includeStreet = true, includePostalCode = true))
    }

    // -------- convenience wrappers --------

    @Test
    fun buildSimpleLocationString_omits_street_and_postal() {
        val addr = address(street = "Hlavna 1", city = "Bratislava", district = "Old Town", postalCode = "81101")
        assertEquals("Old Town, Bratislava", FormatUtils.buildSimpleLocationString(addr))
    }

    @Test
    fun buildDetailedLocationString_includes_street_and_postal() {
        val addr = address(street = "Hlavna 1", city = "Bratislava", district = "Old Town", region = "BA", postalCode = "81101")
        assertEquals(
            "Hlavna 1, 81101, Old Town, Bratislava, BA",
            FormatUtils.buildDetailedLocationString(addr)
        )
    }
}
