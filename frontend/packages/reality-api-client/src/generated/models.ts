

/**
 * Login request
 */
export type Auth_LoginRequest = {
        /**
 * Email address
 */
email: Shared_email
/**
 * Password
 */
password: string
/**
 * 2FA code (if enabled)
 */
twoFactorCode?: string
    };

/**
 * Login response
 */
export type Auth_LoginResponse = {
        /**
 * Access token (JWT)
 */
accessToken: string
/**
 * Refresh token
 */
refreshToken: string
/**
 * Token expiration in seconds
 */
expiresIn: number
/**
 * User info
 */
user: Auth_User
/**
 * Available tenants for this user
 */
tenants: Array<Auth_TenantMembership>
    };

/**
 * Password change request
 */
export type Auth_PasswordChangeRequest = {
        /**
 * Current password
 */
currentPassword: string
/**
 * New password
 */
newPassword: string
    };

/**
 * Password reset request
 */
export type Auth_PasswordResetRequest = {
        /**
 * Email address
 */
email: Shared_email
    };

/**
 * Registration request
 */
export type Auth_RegisterRequest = {
        /**
 * Email address
 */
email: Shared_email
/**
 * Password
 */
password: string
/**
 * Display name
 */
displayName: string
/**
 * Phone number
 */
phone?: Shared_phoneNumber
/**
 * Invitation code (if required)
 */
invitationCode?: string
    };

/**
 * User's membership in a tenant
 */
export type Auth_TenantMembership = {
        /**
 * Tenant ID
 */
tenantId: Shared_uuid
/**
 * Tenant/Organization name
 */
tenantName: string
/**
 * User's role in this tenant
 */
role: Shared_TenantRole
    };

/**
 * 2FA setup response
 */
export type Auth_TwoFactorSetupResponse = {
        /**
 * Secret key for authenticator app
 */
secret: string
/**
 * QR code data URL
 */
qrCodeUrl: string
/**
 * Backup codes
 */
backupCodes: Array<string>
    };

/**
 * User account
 */
export type Auth_User = {
        /**
 * Unique identifier
 */
id: Shared_uuid
/**
 * Email address
 */
email: Shared_email
/**
 * Display name
 */
displayName: string
/**
 * Phone number
 */
phone?: Shared_phoneNumber
/**
 * Profile picture URL
 */
avatarUrl?: string
/**
 * Whether email is verified
 */
emailVerified: boolean
/**
 * Whether 2FA is enabled
 */
twoFactorEnabled: boolean
/**
 * Last login timestamp
 */
lastLoginAt?: Shared_dateTime
/**
 * Account status
 */
status: Auth_UserStatus
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * User account status
 */
export type Auth_UserStatus = 'active' | 'inactive' | 'suspended' | 'pending_verification';

/**
 * Building
 */
export type Buildings_Building = {
        /**
 * Unique identifier
 */
id: Shared_uuid
/**
 * Organization ID
 */
organizationId: Shared_uuid
/**
 * Building name
 */
name: string
/**
 * Building address
 */
address: Shared_Address
/**
 * GPS location
 */
location?: Shared_GeoLocation
/**
 * Building type
 */
type: Buildings_BuildingType
/**
 * Number of floors
 */
floorCount: number
/**
 * Number of units
 */
unitCount: number
/**
 * Year built
 */
yearBuilt?: number
/**
 * Total building area in m2
 */
totalAreaM2?: number
/**
 * Building photo URL
 */
photoUrl?: string
/**
 * Building description
 */
description?: string
/**
 * Building status
 */
status: Buildings_BuildingStatus
/**
 * Manager user ID
 */
managerId?: Shared_uuid
/**
 * Technical manager user ID
 */
technicalManagerId?: Shared_uuid
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Building document
 */
export type Buildings_BuildingDocument = {
        /**
 * Unique identifier
 */
id: Shared_uuid
/**
 * Building ID
 */
buildingId: Shared_uuid
/**
 * Document title
 */
title: string
/**
 * Document category
 */
category: Buildings_DocumentCategory
/**
 * File attachment
 */
file: Shared_Attachment
/**
 * Description
 */
description?: string
/**
 * Visibility
 */
visibility: Buildings_DocumentVisibility
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Building status
 */
export type Buildings_BuildingStatus = 'active' | 'under_construction' | 'renovation' | 'inactive';

/**
 * Building type
 */
export type Buildings_BuildingType = 'residential' | 'commercial' | 'mixed' | 'industrial';

/**
 * Common area (staircase, elevator, parking, garden, etc.)
 */
export type Buildings_CommonArea = {
        /**
 * Unique identifier
 */
id: Shared_uuid
/**
 * Building ID
 */
buildingId: Shared_uuid
/**
 * Area name
 */
name: string
/**
 * Area type
 */
type: Buildings_CommonAreaType
/**
 * Description
 */
description?: string
/**
 * Area size in m2
 */
areaM2?: number
/**
 * Floor ID (if applicable)
 */
floorId?: Shared_uuid
/**
 * Photo URL
 */
photoUrl?: string
    };

/**
 * Common area type
 */
export type Buildings_CommonAreaType = 'staircase' | 'elevator' | 'lobby' | 'hallway' | 'basement' | 'attic' | 'parking' | 'garage' | 'garden' | 'playground' | 'pool' | 'gym' | 'laundry_room' | 'storage_room' | 'other';

/**
 * Create building request
 */
export type Buildings_CreateBuildingRequest = {
        name: string
address: Shared_Address
location?: Shared_GeoLocation
type: Buildings_BuildingType
floorCount: number
yearBuilt?: number
totalAreaM2?: number
description?: string
managerId?: Shared_uuid
technicalManagerId?: Shared_uuid
    };

/**
 * Document category
 */
export type Buildings_DocumentCategory = 'legal_document' | 'insurance' | 'maintenance' | 'financial_report' | 'meeting_minutes' | 'contract' | 'technical_document' | 'other';

/**
 * Document visibility
 */
export type Buildings_DocumentVisibility = 'public' | 'owners_only' | 'managers_only';

/**
 * Floor in a building
 */
export type Buildings_Floor = {
        /**
 * Unique identifier
 */
id: Shared_uuid
/**
 * Building ID
 */
buildingId: Shared_uuid
/**
 * Floor number (0 = ground, -1 = basement)
 */
number: number
/**
 * Floor name/label
 */
name?: string
/**
 * Floor plan URL
 */
floorPlanUrl?: string
/**
 * Number of units on this floor
 */
unitCount: number
    };

/**
 * Audit log entry
 */
export type Compliance_AuditLog = {
        id: Shared_uuid
organizationId: Shared_uuid
userId?: Shared_uuid
action: string
resourceType: string
resourceId?: Shared_uuid
oldValue?: string
newValue?: string
ipAddress?: string
userAgent?: string
timestamp: Shared_dateTime
    };

/**
 * User consent record
 */
export type Compliance_Consent = {
        id: Shared_uuid
userId: Shared_uuid
consentType: Compliance_ConsentType
granted: boolean
grantedAt?: Shared_dateTime
revokedAt?: Shared_dateTime
ipAddress?: string
userAgent?: string
version: string
    };

export type Compliance_ConsentType = 'terms_of_service' | 'privacy_policy' | 'marketing' | 'analytics' | 'third_party_sharing' | 'cookies';

export type Compliance_DataCategory = 'profile' | 'units' | 'documents' | 'messages' | 'votes' | 'faults' | 'payments' | 'audit_logs';

/**
 * GDPR data deletion request
 */
export type Compliance_DataDeletionRequest = {
        id: Shared_uuid
userId: Shared_uuid
status: Compliance_DeletionStatus
reason?: string
requestedAt: Shared_dateTime
scheduledFor: Shared_dateTime
completedAt?: Shared_dateTime
retainedDataCategories?: Array<Compliance_DataCategory>
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * GDPR data export request
 */
export type Compliance_DataExportRequest = {
        id: Shared_uuid
userId: Shared_uuid
status: Compliance_ExportStatus
format: Compliance_ExportFormat
requestedAt: Shared_dateTime
completedAt?: Shared_dateTime
downloadUrl?: Shared_urlString
expiresAt?: Shared_dateTime
includeCategories: Array<Compliance_DataCategory>
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Compliance_DeletionStatus = 'pending' | 'scheduled' | 'processing' | 'completed' | 'cancelled';

export type Compliance_ExportFormat = 'json' | 'csv' | 'pdf';

export type Compliance_ExportStatus = 'pending' | 'processing' | 'completed' | 'expired' | 'failed';

/**
 * Document (UC-08)
 */
export type Documents_Document = {
        id: Shared_uuid
organizationId: Shared_uuid
buildingId?: Shared_uuid
unitId?: Shared_uuid
title: string
description?: string
category: Documents_DocumentCategory
file: Shared_Attachment
visibility: Documents_DocumentVisibility
tags?: Array<string>
expiresAt?: Shared_dateTime
version: number
previousVersionId?: Shared_uuid
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Documents_DocumentCategory = 'contract' | 'invoice' | 'receipt' | 'report' | 'minutes' | 'policy' | 'manual' | 'certificate' | 'permit' | 'insurance' | 'other';

export type Documents_DocumentVisibility = 'public' | 'building_only' | 'owners_only' | 'managers_only' | 'private';

/**
 * Document folder
 */
export type Documents_Folder = {
        id: Shared_uuid
organizationId: Shared_uuid
parentId?: Shared_uuid
name: string
description?: string
visibility: Documents_DocumentVisibility
documentCount: number
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Fault report (UC-03)
 */
export type Faults_Fault = {
        id: Shared_uuid
buildingId: Shared_uuid
unitId?: Shared_uuid
commonAreaId?: Shared_uuid
reporterId: Shared_uuid
title: string
description: string
category: Faults_FaultCategory
priority: Faults_FaultPriority
status: Faults_FaultStatus
photos?: Array<Shared_Attachment>
assignedTo?: Shared_uuid
estimatedCost?: Shared_Money
actualCost?: Shared_Money
scheduledDate?: Shared_dateTime
completedAt?: Shared_dateTime
resolution?: string
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Faults_FaultCategory = 'plumbing' | 'electrical' | 'hvac' | 'structural' | 'elevator' | 'security' | 'cleaning' | 'landscaping' | 'other';

/**
 * Fault comment
 */
export type Faults_FaultComment = {
        id: Shared_uuid
faultId: Shared_uuid
authorId: Shared_uuid
authorName: string
content: string
attachments?: Array<Shared_Attachment>
isInternal: boolean
createdAt: Shared_dateTime
    };

export type Faults_FaultPriority = 'low' | 'medium' | 'high' | 'critical';

export type Faults_FaultStatus = 'reported' | 'acknowledged' | 'in_progress' | 'on_hold' | 'resolved' | 'closed' | 'rejected';

/**
 * Inquiry from potential buyer/renter
 */
export type Listings_Inquiry = {
        id: Shared_uuid
listingId: Shared_uuid
name: string
email: Shared_email
phone?: Shared_phoneNumber
message: string
source: Listings_InquirySource
status: Listings_InquiryStatus
scheduledViewingAt?: Shared_dateTime
notes?: string
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Listings_InquirySource = 'website' | 'portal' | 'phone' | 'email' | 'walk_in';

export type Listings_InquiryStatus = 'new' | 'contacted' | 'viewing_scheduled' | 'viewing_completed' | 'interested' | 'not_interested' | 'closed';

/**
 * Property listing for sale/rent (UC-31-32)
 */
export type Listings_Listing = {
        id: Shared_uuid
unitId: Shared_uuid
type: Listings_ListingType
status: Listings_ListingStatus
title: string
description: string
price: Shared_Money
priceType: Listings_PriceType
features: Listings_ListingFeatures
photos: Array<Shared_Attachment>
virtualTourUrl?: string
videoUrl?: string
agentId?: Shared_uuid
publishedAt?: Shared_dateTime
expiresAt?: Shared_dateTime
viewCount: number
inquiryCount: number
portalListings?: Array<Listings_PortalListing>
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Listings_ListingFeatures = {
        areaM2: number
roomCount?: number
bathroomCount?: number
floorNumber?: number
totalFloors?: number
yearBuilt?: number
hasBalcony?: boolean
hasParking?: boolean
hasElevator?: boolean
hasFurniture?: boolean
petFriendly?: boolean
energyRating?: string
heatingType?: string
amenities?: Array<string>
    };

export type Listings_ListingStatus = 'draft' | 'active' | 'pending' | 'sold' | 'rented' | 'expired' | 'archived';

export type Listings_ListingType = 'sale' | 'long_term_rent' | 'short_term_rent';

/**
 * Real estate portal configuration
 */
export type Listings_PortalConfig = {
        id: Shared_uuid
organizationId: Shared_uuid
portalName: string
apiEndpoint: string
apiKey: string
isActive: boolean
autoPublish: boolean
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Listing on external portal
 */
export type Listings_PortalListing = {
        portalId: Shared_uuid
portalName: string
externalId: string
externalUrl?: string
status: Listings_PortalListingStatus
lastSyncAt?: Shared_dateTime
    };

export type Listings_PortalListingStatus = 'published' | 'pending' | 'error' | 'removed';

export type Listings_PriceType = 'fixed' | 'negotiable' | 'per_month' | 'per_night';

/**
 * Create organization request
 */
export type Organizations_CreateOrganizationRequest = {
        name: string
legalName?: string
taxId?: string
registrationNumber?: string
address: Shared_Address
email: Shared_email
phone?: Shared_phoneNumber
website?: string
type: Organizations_OrganizationType
    };

/**
 * Invite member request
 */
export type Organizations_InviteMemberRequest = {
        email: Shared_email
role: Shared_TenantRole
assignedBuildingIds?: Array<Shared_uuid>
message?: string
    };

/**
 * Member status
 */
export type Organizations_MemberStatus = 'active' | 'invited' | 'suspended';

/**
 * Organization (housing cooperative, property management company)
 */
export type Organizations_Organization = {
        /**
 * Unique identifier
 */
id: Shared_uuid
/**
 * Organization name
 */
name: string
/**
 * Legal name
 */
legalName?: string
/**
 * Tax/VAT ID
 */
taxId?: string
/**
 * Registration number
 */
registrationNumber?: string
/**
 * Headquarters address
 */
address: Shared_Address
/**
 * Contact email
 */
email: Shared_email
/**
 * Contact phone
 */
phone?: Shared_phoneNumber
/**
 * Website URL
 */
website?: string
/**
 * Logo URL
 */
logoUrl?: string
/**
 * Organization type
 */
type: Organizations_OrganizationType
/**
 * Subscription tier
 */
subscriptionTier: Organizations_SubscriptionTier
/**
 * Organization status
 */
status: Organizations_OrganizationStatus
/**
 * Custom branding settings
 */
branding?: Organizations_OrganizationBranding
/**
 * Number of buildings
 */
buildingCount: number
/**
 * Number of units
 */
unitCount: number
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Organization branding settings
 */
export type Organizations_OrganizationBranding = {
        /**
 * Primary color (hex)
 */
primaryColor?: string
/**
 * Secondary color (hex)
 */
secondaryColor?: string
/**
 * Custom logo URL
 */
logoUrl?: string
/**
 * Custom favicon URL
 */
faviconUrl?: string
/**
 * Custom email footer
 */
emailFooter?: string
    };

/**
 * Organization member
 */
export type Organizations_OrganizationMember = {
        /**
 * User ID
 */
userId: Shared_uuid
/**
 * User email
 */
email: Shared_email
/**
 * User display name
 */
displayName: string
/**
 * Role in organization
 */
role: Shared_TenantRole
/**
 * Assigned buildings (empty = all)
 */
assignedBuildingIds: Array<Shared_uuid>
/**
 * Joined timestamp
 */
joinedAt: Shared_dateTime
/**
 * Membership status
 */
status: Organizations_MemberStatus
    };

/**
 * Organization status
 */
export type Organizations_OrganizationStatus = 'active' | 'trial' | 'suspended' | 'cancelled';

/**
 * Organization type
 */
export type Organizations_OrganizationType = 'housing_cooperative' | 'property_management' | 'real_estate_agency' | 'individual';

/**
 * Subscription tier
 */
export type Organizations_SubscriptionTier = 'free' | 'starter' | 'professional' | 'enterprise';

export type Rentals_DocumentType = 'passport' | 'id_card' | 'drivers_license' | 'other';

/**
 * Guest registration for police/authorities
 */
export type Rentals_GuestRegistration = {
        id: Shared_uuid
reservationId?: Shared_uuid
unitId: Shared_uuid
firstName: string
lastName: string
dateOfBirth: Shared_date
nationality: string
documentType: Rentals_DocumentType
documentNumber: string
documentExpiry?: Shared_date
documentScanUrl?: string
arrivalDate: Shared_date
departureDate: Shared_date
submittedToAuthorities: boolean
submittedAt?: Shared_dateTime
authorityReference?: string
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Platform connection settings
 */
export type Rentals_PlatformConnection = {
        id: Shared_uuid
unitId: Shared_uuid
platform: Rentals_RentalPlatform
externalListingId?: string
apiKey?: string
isActive: boolean
lastSyncAt?: Shared_dateTime
syncStatus: Rentals_SyncStatus
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Rentals_RentalPlatform = 'airbnb' | 'booking' | 'vrbo' | 'direct' | 'other';

/**
 * Short-term rental reservation (UC-29-30)
 */
export type Rentals_Reservation = {
        id: Shared_uuid
unitId: Shared_uuid
platform: Rentals_RentalPlatform
externalId: string
guestName: string
guestEmail?: Shared_email
guestPhone?: Shared_phoneNumber
guestCount: number
checkIn: Shared_dateTime
checkOut: Shared_dateTime
status: Rentals_ReservationStatus
totalPrice: Shared_Money
platformFee?: Shared_Money
cleaningFee?: Shared_Money
accessCode?: string
notes?: string
guestRegistrationId?: Shared_uuid
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Rentals_ReservationStatus = 'pending' | 'confirmed' | 'checked_in' | 'checked_out' | 'cancelled' | 'no_show';

export type Rentals_SyncStatus = 'synced' | 'pending' | 'error' | 'disabled';

/**
 * Physical address
 */
export type Shared_Address = {
        /**
 * Street address line 1
 */
street1: string
/**
 * Street address line 2 (optional)
 */
street2?: string
/**
 * City name
 */
city: string
/**
 * State or province
 */
state?: string
/**
 * Postal/ZIP code
 */
postalCode: string
/**
 * ISO 3166-1 alpha-2 country code
 */
country: string
    };

/**
 * File attachment metadata
 */
export type Shared_Attachment = {
        /**
 * Unique identifier
 */
id: Shared_uuid
/**
 * Original filename
 */
filename: string
/**
 * MIME type
 */
mimeType: string
/**
 * File size in bytes
 */
sizeBytes: number
/**
 * Download URL
 */
downloadUrl: Shared_urlString
/**
 * Upload timestamp
 */
uploadedAt: Shared_dateTime
    };

/**
 * Base model for all entities with audit fields
 */
export type Shared_AuditableEntity = {
        /**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Authenticated user info (from JWT)
 */
export type Shared_AuthenticatedUser = {
        /**
 * User ID
 */
userId: Shared_uuid
/**
 * Current tenant ID
 */
tenantId: Shared_uuid
/**
 * User's role in current tenant
 */
role: Shared_TenantRole
/**
 * User's email
 */
email: Shared_email
/**
 * User's display name
 */
displayName: string
/**
 * Active delegations to this user
 */
delegations?: Array<Shared_Delegation>
    };

/**
 * Cursor-based pagination metadata
 */
export type Shared_CursorPaginationMeta = {
        /**
 * Cursor for next page (null if no more items)
 */
nextCursor?: string
/**
 * Has more items
 */
hasMore: boolean
    };

/**
 * Cursor-based pagination query
 */
export type Shared_CursorPaginationQuery = {
        /**
 * Cursor for next page
 */
cursor?: string
/**
 * Items per page
 */
limit?: number
    };

/**
 * Delegation of rights
 */
export type Shared_Delegation = {
        /**
 * Unique identifier
 */
id: Shared_uuid
/**
 * User granting the delegation
 */
fromUserId: Shared_uuid
/**
 * User receiving the delegation
 */
toUserId: Shared_uuid
/**
 * Delegated resource accesses
 */
accesses: Array<Shared_ResourceAccess>
/**
 * Delegation start date
 */
startsAt: Shared_dateTime
/**
 * Delegation end date (null for permanent)
 */
endsAt?: Shared_dateTime
/**
 * Whether delegation is currently active
 */
isActive: boolean
/**
 * Reason for delegation
 */
reason?: string
    };

/**
 * Standard error response
 */
export type Shared_ErrorResponse = {
        /**
 * Error code for programmatic handling
 */
code: string
/**
 * Human-readable error message
 */
message: string
/**
 * Request ID for support/debugging
 */
requestId?: string
/**
 * Detailed validation errors
 */
details?: Array<Shared_ValidationError>
/**
 * ISO 8601 timestamp
 */
timestamp: Shared_dateTime
    };

/**
 * GPS coordinates
 */
export type Shared_GeoLocation = {
        /**
 * Latitude (-90 to 90)
 */
latitude: number
/**
 * Longitude (-180 to 180)
 */
longitude: number
    };

/**
 * Internal Server Error
 */
export type Shared_InternalServerError = {
        body: Shared_ErrorResponse
    };

/**
 * Localized content
 */
export type Shared_LocalizedString = {
        /**
 * Default language value
 */
default: string
/**
 * Translations by ISO 639-1 language code
 */
translations?: Record<string, string>
    };

/**
 * Monetary amount with currency
 */
export type Shared_Money = {
        /**
 * Amount in smallest currency unit (e.g., cents)
 */
amount: number
/**
 * ISO 4217 currency code
 */
currency: string
    };

/**
 * Pagination metadata
 */
export type Shared_PaginationMeta = {
        /**
 * Current page number
 */
page: number
/**
 * Items per page
 */
limit: number
/**
 * Total number of items
 */
totalItems: number
/**
 * Total number of pages
 */
totalPages: number
/**
 * Has next page
 */
hasNext: boolean
/**
 * Has previous page
 */
hasPrevious: boolean
    };

/**
 * Permission scope
 */
export type Shared_PermissionScope = 'read' | 'create' | 'update' | 'delete' | 'admin';

/**
 * Too Many Requests - Rate limit exceeded
 */
export type Shared_RateLimitError = {
        body: Shared_ErrorResponse
    };

/**
 * Resource access level
 */
export type Shared_ResourceAccess = {
        /**
 * Resource type (e.g., 'building', 'unit', 'document')
 */
resourceType: string
/**
 * Resource ID (null for all resources of type)
 */
resourceId?: Shared_uuid
/**
 * Granted permissions
 */
permissions: Array<Shared_PermissionScope>
/**
 * Access expiration (null for permanent)
 */
expiresAt?: Shared_dateTime
    };

/**
 * Tenant context - required for all multi-tenant operations
 */
export type Shared_TenantContext = Record<string, unknown>;

/**
 * User role within tenant
 */
export type Shared_TenantRole = 'super_admin' | 'org_admin' | 'manager' | 'technical_manager' | 'owner' | 'owner_delegate' | 'tenant' | 'resident' | 'property_manager' | 'real_estate_agent' | 'guest';

/**
 * Unprocessable Entity - Business rule violation
 */
export type Shared_UnprocessableEntityError = {
        body: Shared_ErrorResponse
    };

/**
 * Validation error detail
 */
export type Shared_ValidationError = {
        /**
 * Field path (e.g., 'address.city')
 */
field: string
/**
 * Error message for this field
 */
message: string
/**
 * Error code
 */
code: string
    };

/**
 * ISO 8601 date string (YYYY-MM-DD)
 */
export type Shared_date = string;

/**
 * ISO 8601 date-time string
 */
export type Shared_dateTime = string;

/**
 * Email address
 */
export type Shared_email = string;

/**
 * Phone number in E.164 format
 */
export type Shared_phoneNumber = string;

/**
 * URL string
 */
export type Shared_urlString = string;

/**
 * Unique identifier (UUID v4)
 */
export type Shared_uuid = string;

export type Units_ResidentType = 'owner' | 'tenant' | 'family_member' | 'subtenant';

/**
 * Property unit (apartment, office, etc.)
 */
export type Units_Unit = {
        id: Shared_uuid
buildingId: Shared_uuid
floorId?: Shared_uuid
unitNumber: string
type: Units_UnitType
areaM2: number
roomCount?: number
bathroomCount?: number
balconyCount?: number
hasParking: boolean
hasStorage: boolean
status: Units_UnitStatus
ownershipShare?: number
description?: string
photoUrls?: Array<string>
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Unit owner
 */
export type Units_UnitOwner = {
        id: Shared_uuid
unitId: Shared_uuid
userId?: Shared_uuid
name: string
email?: Shared_email
phone?: Shared_phoneNumber
ownershipPercentage: number
isPrimaryOwner: boolean
acquiredAt?: Shared_date
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

/**
 * Unit resident
 */
export type Units_UnitResident = {
        id: Shared_uuid
unitId: Shared_uuid
userId?: Shared_uuid
name: string
email?: Shared_email
phone?: Shared_phoneNumber
type: Units_ResidentType
moveInDate?: Shared_date
moveOutDate?: Shared_date
isActive: boolean
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Units_UnitStatus = 'occupied' | 'vacant' | 'for_sale' | 'for_rent' | 'renovation';

export type Units_UnitType = 'apartment' | 'studio' | 'penthouse' | 'office' | 'retail' | 'storage' | 'parking';

export type Voting_Ballot = {
        id: Shared_uuid
voteId: Shared_uuid
voterId: Shared_uuid
delegatedFrom?: Shared_uuid
selectedOptionIds: Array<Shared_uuid>
weight: number
submittedAt: Shared_dateTime
    };

/**
 * Vote/Poll (UC-04)
 */
export type Voting_Vote = {
        id: Shared_uuid
buildingId: Shared_uuid
title: string
description: string
type: Voting_VoteType
status: Voting_VoteStatus
startDate: Shared_dateTime
endDate: Shared_dateTime
quorumPercentage: number
options: Array<Voting_VoteOption>
allowDelegation: boolean
isAnonymous: boolean
/**
 * Creation timestamp
 */
createdAt: Shared_dateTime
/**
 * Last update timestamp
 */
updatedAt: Shared_dateTime
/**
 * ID of user who created this entity
 */
createdBy?: Shared_uuid
/**
 * ID of user who last updated this entity
 */
updatedBy?: Shared_uuid
    };

export type Voting_VoteOption = {
        id: Shared_uuid
text: string
description?: string
order: number
    };

export type Voting_VoteOptionResult = {
        optionId: Shared_uuid
text: string
voteCount: number
weightedVotes: number
percentage: number
    };

export type Voting_VoteResult = {
        voteId: Shared_uuid
totalEligibleVoters: number
totalVotes: number
participationRate: number
quorumReached: boolean
optionResults: Array<Voting_VoteOptionResult>
winningOptionId?: Shared_uuid
    };

export type Voting_VoteStatus = 'draft' | 'scheduled' | 'active' | 'closed' | 'cancelled';

export type Voting_VoteType = 'simple' | 'weighted' | 'ranked_choice';

/**
 * Items per page
 */
export type Shared_PaginationQuery_limit = number;

/**
 * Page number (1-indexed)
 */
export type Shared_PaginationQuery_page = number;

/**
 * Sort field
 */
export type Shared_PaginationQuery_sortBy = string;

/**
 * Sort direction
 */
export type Shared_PaginationQuery_sortOrder = 'asc' | 'desc';

/**
 * Filter by date from (ISO 8601)
 */
export type Shared_SearchQuery_from = Shared_dateTime;

/**
 * Search term
 */
export type Shared_SearchQuery_q = string;

/**
 * Filter by status
 */
export type Shared_SearchQuery_status = string;

/**
 * Filter by date to (ISO 8601)
 */
export type Shared_SearchQuery_to = Shared_dateTime;

export type AuthenticationData = {
        
        payloads: {
            AuthApiLogin: {
                        requestBody: Auth_LoginRequest
                        
                    };
AuthApiLogout: {
                        authorization: string
                        
                    };
AuthApiGetMe: {
                        authorization: string
                        
                    };
AuthApiUpdateMe: {
                        authorization: string
requestBody: {
        displayName?: string
phone?: Shared_phoneNumber
avatarUrl?: string
    }
                        
                    };
AuthApiDisable2Fa: {
                        authorization: string
requestBody: {
        code: string
    }
                        
                    };
AuthApiEnable2Fa: {
                        authorization: string
requestBody: {
        code: string
    }
                        
                    };
AuthApiSetup2Fa: {
                        authorization: string
                        
                    };
AuthApiChangePassword: {
                        authorization: string
requestBody: Auth_PasswordChangeRequest
                        
                    };
AuthApiRequestPasswordReset: {
                        requestBody: Auth_PasswordResetRequest
                        
                    };
AuthApiRefresh: {
                        authorization: string
                        
                    };
AuthApiRegister: {
                        requestBody: Auth_RegisterRequest
                        
                    };
        }
        
        
        responses: {
            AuthApiLogin: Auth_LoginResponse
                ,AuthApiLogout: void
                ,AuthApiGetMe: Auth_User
                ,AuthApiUpdateMe: Auth_User
                ,AuthApiDisable2Fa: void
                ,AuthApiEnable2Fa: void
                ,AuthApiSetup2Fa: Auth_TwoFactorSetupResponse
                ,AuthApiChangePassword: void
                ,AuthApiRequestPasswordReset: any
                ,AuthApiRefresh: Auth_LoginResponse
                ,AuthApiRegister: Auth_User
                
        }
        
    }

export type BuildingsData = {
        
        payloads: {
            BuildingsApiList: {
                        authorization: string
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
status?: Buildings_BuildingStatus
type?: Buildings_BuildingType
xTenantId: Shared_uuid
                        
                    };
BuildingsApiCreate: {
                        authorization: string
requestBody: Buildings_CreateBuildingRequest
xTenantId: Shared_uuid
                        
                    };
BuildingsApiGet: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
BuildingsApiUpdate: {
                        authorization: string
id: Shared_uuid
requestBody: {
        name?: string
address?: Shared_Address
location?: Shared_GeoLocation
type?: Buildings_BuildingType
floorCount?: number
yearBuilt?: number
totalAreaM2?: number
description?: string
status?: Buildings_BuildingStatus
managerId?: Shared_uuid
technicalManagerId?: Shared_uuid
    }
xTenantId: Shared_uuid
                        
                    };
BuildingsApiDelete: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
BuildingsApiListCommonAreas: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
BuildingsApiCreateCommonArea: {
                        authorization: string
id: Shared_uuid
requestBody: {
        name: string
type: Buildings_CommonAreaType
description?: string
areaM2?: number
floorId?: Shared_uuid
    }
xTenantId: Shared_uuid
                        
                    };
BuildingsApiListDocuments: {
                        authorization: string
category?: Buildings_DocumentCategory
id: Shared_uuid
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
xTenantId: Shared_uuid
                        
                    };
BuildingsApiUploadDocument: {
                        authorization: string
id: Shared_uuid
requestBody: {
        title: string
category: Buildings_DocumentCategory
description?: string
visibility: Buildings_DocumentVisibility
/**
 * Base64 encoded file content
 */
fileContent: string
fileName: string
mimeType: string
    }
xTenantId: Shared_uuid
                        
                    };
BuildingsApiListFloors: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
BuildingsApiCreateFloor: {
                        authorization: string
id: Shared_uuid
requestBody: {
        number: number
name?: string
    }
xTenantId: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            BuildingsApiList: {
        /**
 * Array of items
 */
data: Array<Buildings_Building>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,BuildingsApiCreate: Buildings_Building
                ,BuildingsApiGet: Buildings_Building
                ,BuildingsApiUpdate: Buildings_Building
                ,BuildingsApiDelete: void
                ,BuildingsApiListCommonAreas: Array<Buildings_CommonArea>
                ,BuildingsApiCreateCommonArea: Buildings_CommonArea
                ,BuildingsApiListDocuments: {
        /**
 * Array of items
 */
data: Array<Buildings_BuildingDocument>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,BuildingsApiUploadDocument: Buildings_BuildingDocument
                ,BuildingsApiListFloors: Array<Buildings_Floor>
                ,BuildingsApiCreateFloor: Buildings_Floor
                
        }
        
    }

export type ComplianceGdprData = {
        
        payloads: {
            ComplianceApiGetAuditLogs: {
                        action?: string
authorization: string
from?: Shared_dateTime
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
resourceType?: string
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
to?: Shared_dateTime
userId?: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
ComplianceApiGetConsents: {
                        authorization: string
                        
                    };
ComplianceApiUpdateConsent: {
                        authorization: string
requestBody: {
        consentType: Compliance_ConsentType
granted: boolean
    }
                        
                    };
ComplianceApiRequestDeletion: {
                        authorization: string
requestBody: {
        reason?: string
retainedDataCategories?: Array<Compliance_DataCategory>
    }
                        
                    };
ComplianceApiGetDeletionStatus: {
                        authorization: string
id: Shared_uuid
                        
                    };
ComplianceApiCancelDeletion: {
                        authorization: string
id: Shared_uuid
                        
                    };
ComplianceApiRequestExport: {
                        authorization: string
requestBody: {
        format?: Compliance_ExportFormat
includeCategories?: Array<Compliance_DataCategory>
    }
                        
                    };
ComplianceApiGetExportStatus: {
                        authorization: string
id: Shared_uuid
                        
                    };
ComplianceApiDownloadExport: {
                        authorization: string
id: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            ComplianceApiGetAuditLogs: {
        /**
 * Array of items
 */
data: Array<Compliance_AuditLog>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,ComplianceApiGetConsents: Array<Compliance_Consent>
                ,ComplianceApiUpdateConsent: Compliance_Consent
                ,ComplianceApiRequestDeletion: Compliance_DataDeletionRequest
                ,ComplianceApiGetDeletionStatus: Compliance_DataDeletionRequest
                ,ComplianceApiCancelDeletion: Compliance_DataDeletionRequest
                ,ComplianceApiRequestExport: Compliance_DataExportRequest
                ,ComplianceApiGetExportStatus: Compliance_DataExportRequest
                ,ComplianceApiDownloadExport: Blob | File
                
        }
        
    }

export type DocumentsData = {
        
        payloads: {
            DocumentsApiList: {
                        authorization: string
buildingId?: Shared_uuid
category?: Documents_DocumentCategory
folderId?: Shared_uuid
/**
 * Filter by date from (ISO 8601)
 */
from?: Shared_dateTime
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Search term
 */
q?: string
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
/**
 * Filter by status
 */
status?: string
/**
 * Filter by date to (ISO 8601)
 */
to?: Shared_dateTime
xTenantId: Shared_uuid
                        
                    };
DocumentsApiUpload: {
                        authorization: string
requestBody: {
        title: string
description?: string
category: Documents_DocumentCategory
buildingId?: Shared_uuid
unitId?: Shared_uuid
folderId?: Shared_uuid
visibility: Documents_DocumentVisibility
tags?: Array<string>
fileContent: string
fileName: string
mimeType: string
    }
xTenantId: Shared_uuid
                        
                    };
DocumentsApiListFolders: {
                        authorization: string
parentId?: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
DocumentsApiCreateFolder: {
                        authorization: string
requestBody: {
        name: string
description?: string
parentId?: Shared_uuid
visibility: Documents_DocumentVisibility
    }
xTenantId: Shared_uuid
                        
                    };
DocumentsApiGet: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
DocumentsApiUpdate: {
                        authorization: string
id: Shared_uuid
requestBody: {
        title?: string
description?: string
category?: Documents_DocumentCategory
visibility?: Documents_DocumentVisibility
tags?: Array<string>
    }
xTenantId: Shared_uuid
                        
                    };
DocumentsApiDelete: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
DocumentsApiDownload: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
DocumentsApiUploadNewVersion: {
                        authorization: string
id: Shared_uuid
requestBody: {
        fileContent: string
fileName: string
mimeType: string
    }
xTenantId: Shared_uuid
                        
                    };
DocumentsApiListVersions: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            DocumentsApiList: {
        /**
 * Array of items
 */
data: Array<Documents_Document>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,DocumentsApiUpload: Documents_Document
                ,DocumentsApiListFolders: Array<Documents_Folder>
                ,DocumentsApiCreateFolder: Documents_Folder
                ,DocumentsApiGet: Documents_Document
                ,DocumentsApiUpdate: Documents_Document
                ,DocumentsApiDelete: void
                ,DocumentsApiDownload: Blob | File
                ,DocumentsApiUploadNewVersion: Documents_Document
                ,DocumentsApiListVersions: Array<Documents_Document>
                
        }
        
    }

export type FaultsData = {
        
        payloads: {
            FaultsApiList: {
                        authorization: string
buildingId?: Shared_uuid
category?: Faults_FaultCategory
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
priority?: Faults_FaultPriority
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
status?: Faults_FaultStatus
xTenantId: Shared_uuid
                        
                    };
FaultsApiCreate: {
                        authorization: string
requestBody: {
        buildingId: Shared_uuid
unitId?: Shared_uuid
commonAreaId?: Shared_uuid
title: string
description: string
category: Faults_FaultCategory
priority?: Faults_FaultPriority
    }
xTenantId: Shared_uuid
                        
                    };
FaultsApiGet: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
FaultsApiUpdate: {
                        authorization: string
id: Shared_uuid
requestBody: {
        title?: string
description?: string
category?: Faults_FaultCategory
priority?: Faults_FaultPriority
status?: Faults_FaultStatus
assignedTo?: Shared_uuid
estimatedCost?: Shared_Money
scheduledDate?: Shared_dateTime
    }
xTenantId: Shared_uuid
                        
                    };
FaultsApiListComments: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
FaultsApiAddComment: {
                        authorization: string
id: Shared_uuid
requestBody: {
        content: string
isInternal?: boolean
    }
xTenantId: Shared_uuid
                        
                    };
FaultsApiUploadPhoto: {
                        authorization: string
id: Shared_uuid
requestBody: {
        fileContent: string
fileName: string
mimeType: string
    }
xTenantId: Shared_uuid
                        
                    };
FaultsApiResolve: {
                        authorization: string
id: Shared_uuid
requestBody: {
        resolution: string
actualCost?: Shared_Money
    }
xTenantId: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            FaultsApiList: {
        /**
 * Array of items
 */
data: Array<Faults_Fault>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,FaultsApiCreate: Faults_Fault
                ,FaultsApiGet: Faults_Fault
                ,FaultsApiUpdate: Faults_Fault
                ,FaultsApiListComments: Array<Faults_FaultComment>
                ,FaultsApiAddComment: Faults_FaultComment
                ,FaultsApiUploadPhoto: Shared_Attachment
                ,FaultsApiResolve: Faults_Fault
                
        }
        
    }

export type RealEstateListingsData = {
        
        payloads: {
            ListingsApiList: {
                        authorization: string
/**
 * Items per page
 */
limit?: number
maxPrice?: number
minPrice?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
status?: Listings_ListingStatus
type?: Listings_ListingType
xTenantId: Shared_uuid
                        
                    };
ListingsApiCreate: {
                        authorization: string
requestBody: {
        unitId: Shared_uuid
type: Listings_ListingType
title: string
description: string
price: Shared_Money
priceType: Listings_PriceType
features: Listings_ListingFeatures
    }
xTenantId: Shared_uuid
                        
                    };
ListingsApiUpdateInquiry: {
                        authorization: string
id: Shared_uuid
requestBody: {
        status?: Listings_InquiryStatus
scheduledViewingAt?: Shared_dateTime
notes?: string
    }
xTenantId: Shared_uuid
                        
                    };
ListingsApiListPortals: {
                        authorization: string
xTenantId: Shared_uuid
                        
                    };
ListingsApiConfigurePortal: {
                        authorization: string
requestBody: {
        portalName: string
apiEndpoint: string
apiKey: string
autoPublish?: boolean
    }
xTenantId: Shared_uuid
                        
                    };
ListingsApiGet: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
ListingsApiUpdate: {
                        authorization: string
id: Shared_uuid
requestBody: {
        title?: string
description?: string
price?: Shared_Money
features?: Listings_ListingFeatures
    }
xTenantId: Shared_uuid
                        
                    };
ListingsApiListInquiries: {
                        authorization: string
id: Shared_uuid
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
xTenantId: Shared_uuid
                        
                    };
ListingsApiCreateInquiry: {
                        authorization: string
id: Shared_uuid
requestBody: {
        name: string
email: Shared_email
phone?: Shared_phoneNumber
message: string
source?: Listings_InquirySource
    }
xTenantId: Shared_uuid
                        
                    };
ListingsApiUploadPhoto: {
                        authorization: string
id: Shared_uuid
requestBody: {
        fileContent: string
fileName: string
mimeType: string
    }
xTenantId: Shared_uuid
                        
                    };
ListingsApiPublish: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
ListingsApiPublishToPortal: {
                        authorization: string
id: Shared_uuid
requestBody: {
        portalId: Shared_uuid
    }
xTenantId: Shared_uuid
                        
                    };
ListingsApiUnpublish: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            ListingsApiList: {
        /**
 * Array of items
 */
data: Array<Listings_Listing>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,ListingsApiCreate: Listings_Listing
                ,ListingsApiUpdateInquiry: Listings_Inquiry
                ,ListingsApiListPortals: Array<Listings_PortalConfig>
                ,ListingsApiConfigurePortal: Listings_PortalConfig
                ,ListingsApiGet: Listings_Listing
                ,ListingsApiUpdate: Listings_Listing
                ,ListingsApiListInquiries: {
        /**
 * Array of items
 */
data: Array<Listings_Inquiry>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,ListingsApiCreateInquiry: Listings_Inquiry
                ,ListingsApiUploadPhoto: Shared_Attachment
                ,ListingsApiPublish: Listings_Listing
                ,ListingsApiPublishToPortal: Listings_PortalListing
                ,ListingsApiUnpublish: Listings_Listing
                
        }
        
    }

export type OrganizationsData = {
        
        payloads: {
            OrganizationsApiList: {
                        authorization: string
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
status?: Organizations_OrganizationStatus
                        
                    };
OrganizationsApiCreate: {
                        authorization: string
requestBody: Organizations_CreateOrganizationRequest
                        
                    };
OrganizationsApiSwitchTenant: {
                        authorization: string
id: Shared_uuid
                        
                    };
OrganizationsApiGet: {
                        authorization: string
id: Shared_uuid
                        
                    };
OrganizationsApiUpdate: {
                        authorization: string
id: Shared_uuid
requestBody: {
        name?: string
legalName?: string
taxId?: string
registrationNumber?: string
address?: Shared_Address
email?: Shared_email
phone?: Shared_phoneNumber
website?: string
    }
                        
                    };
OrganizationsApiDelete: {
                        authorization: string
id: Shared_uuid
                        
                    };
OrganizationsApiUpdateBranding: {
                        authorization: string
id: Shared_uuid
requestBody: Organizations_OrganizationBranding
                        
                    };
OrganizationsApiListMembers: {
                        authorization: string
id: Shared_uuid
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
                        
                    };
OrganizationsApiInviteMember: {
                        authorization: string
id: Shared_uuid
requestBody: Organizations_InviteMemberRequest
                        
                    };
OrganizationsApiUpdateMember: {
                        authorization: string
id: Shared_uuid
requestBody: {
        role?: Shared_TenantRole
assignedBuildingIds?: Array<Shared_uuid>
    }
userId: Shared_uuid
                        
                    };
OrganizationsApiRemoveMember: {
                        authorization: string
id: Shared_uuid
userId: Shared_uuid
                        
                    };
OrganizationsApiGetStats: {
                        authorization: string
id: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            OrganizationsApiList: {
        /**
 * Array of items
 */
data: Array<Organizations_Organization>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,OrganizationsApiCreate: Organizations_Organization
                ,OrganizationsApiSwitchTenant: {
        accessToken: string
organization: Organizations_Organization
    }
                ,OrganizationsApiGet: Organizations_Organization
                ,OrganizationsApiUpdate: Organizations_Organization
                ,OrganizationsApiDelete: void
                ,OrganizationsApiUpdateBranding: Organizations_Organization
                ,OrganizationsApiListMembers: {
        /**
 * Array of items
 */
data: Array<Organizations_OrganizationMember>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,OrganizationsApiInviteMember: Organizations_OrganizationMember
                ,OrganizationsApiUpdateMember: Organizations_OrganizationMember
                ,OrganizationsApiRemoveMember: void
                ,OrganizationsApiGetStats: {
        buildingCount: number
unitCount: number
memberCount: number
activeIssues: number
pendingVotes: number
    }
                
        }
        
    }

export type ShortTermRentalsData = {
        
        payloads: {
            RentalsApiListConnections: {
                        authorization: string
unitId?: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
RentalsApiCreateConnection: {
                        authorization: string
requestBody: {
        unitId: Shared_uuid
platform: Rentals_RentalPlatform
externalListingId?: string
apiKey?: string
    }
xTenantId: Shared_uuid
                        
                    };
RentalsApiListGuests: {
                        authorization: string
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
unitId?: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
RentalsApiRegisterGuest: {
                        authorization: string
requestBody: {
        reservationId?: Shared_uuid
unitId: Shared_uuid
firstName: string
lastName: string
dateOfBirth: Shared_date
nationality: string
documentType: Rentals_DocumentType
documentNumber: string
documentExpiry?: Shared_date
arrivalDate: Shared_date
departureDate: Shared_date
    }
xTenantId: Shared_uuid
                        
                    };
RentalsApiSubmitToAuthorities: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
RentalsApiListReservations: {
                        authorization: string
from?: Shared_date
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
status?: Rentals_ReservationStatus
to?: Shared_date
unitId?: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
RentalsApiCreateReservation: {
                        authorization: string
requestBody: {
        unitId: Shared_uuid
platform: Rentals_RentalPlatform
guestName: string
guestEmail?: Shared_email
guestPhone?: Shared_phoneNumber
guestCount: number
checkIn: Shared_dateTime
checkOut: Shared_dateTime
totalPrice: Shared_Money
    }
xTenantId: Shared_uuid
                        
                    };
RentalsApiGetReservation: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
RentalsApiCheckIn: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
RentalsApiCheckOut: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
RentalsApiGenerateAccessCode: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
RentalsApiSyncPlatforms: {
                        authorization: string
unitId?: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            RentalsApiListConnections: Array<Rentals_PlatformConnection>
                ,RentalsApiCreateConnection: Rentals_PlatformConnection
                ,RentalsApiListGuests: {
        /**
 * Array of items
 */
data: Array<Rentals_GuestRegistration>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,RentalsApiRegisterGuest: Rentals_GuestRegistration
                ,RentalsApiSubmitToAuthorities: Rentals_GuestRegistration
                ,RentalsApiListReservations: {
        /**
 * Array of items
 */
data: Array<Rentals_Reservation>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,RentalsApiCreateReservation: Rentals_Reservation
                ,RentalsApiGetReservation: Rentals_Reservation
                ,RentalsApiCheckIn: Rentals_Reservation
                ,RentalsApiCheckOut: Rentals_Reservation
                ,RentalsApiGenerateAccessCode: {
        accessCode: string
expiresAt: Shared_dateTime
    }
                ,RentalsApiSyncPlatforms: {
        syncedCount: number
errors: Array<string>
    }
                
        }
        
    }

export type UnitsData = {
        
        payloads: {
            UnitsApiList: {
                        authorization: string
buildingId?: Shared_uuid
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
status?: Units_UnitStatus
xTenantId: Shared_uuid
                        
                    };
UnitsApiCreate: {
                        authorization: string
requestBody: {
        buildingId: Shared_uuid
floorId?: Shared_uuid
unitNumber: string
type: Units_UnitType
areaM2: number
roomCount?: number
bathroomCount?: number
    }
xTenantId: Shared_uuid
                        
                    };
UnitsApiGet: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
UnitsApiUpdate: {
                        authorization: string
id: Shared_uuid
requestBody: {
        unitNumber?: string
areaM2?: number
status?: Units_UnitStatus
    }
xTenantId: Shared_uuid
                        
                    };
UnitsApiListOwners: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
UnitsApiAddOwner: {
                        authorization: string
id: Shared_uuid
requestBody: {
        userId?: Shared_uuid
name: string
email?: Shared_email
ownershipPercentage: number
    }
xTenantId: Shared_uuid
                        
                    };
UnitsApiListResidents: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
UnitsApiAddResident: {
                        authorization: string
id: Shared_uuid
requestBody: {
        userId?: Shared_uuid
name: string
type: Units_ResidentType
moveInDate?: Shared_date
    }
xTenantId: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            UnitsApiList: {
        /**
 * Array of items
 */
data: Array<Units_Unit>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,UnitsApiCreate: Units_Unit
                ,UnitsApiGet: Units_Unit
                ,UnitsApiUpdate: Units_Unit
                ,UnitsApiListOwners: Array<Units_UnitOwner>
                ,UnitsApiAddOwner: Units_UnitOwner
                ,UnitsApiListResidents: Array<Units_UnitResident>
                ,UnitsApiAddResident: Units_UnitResident
                
        }
        
    }

export type VotingData = {
        
        payloads: {
            VotingApiList: {
                        authorization: string
buildingId?: Shared_uuid
/**
 * Items per page
 */
limit?: number
/**
 * Page number (1-indexed)
 */
page?: number
/**
 * Sort field
 */
sortBy?: string
/**
 * Sort direction
 */
sortOrder?: 'asc' | 'desc'
status?: Voting_VoteStatus
xTenantId: Shared_uuid
                        
                    };
VotingApiCreate: {
                        authorization: string
requestBody: {
        buildingId: Shared_uuid
title: string
description: string
type: Voting_VoteType
startDate: Shared_dateTime
endDate: Shared_dateTime
quorumPercentage?: number
options: Array<{
        text: string
description?: string
    }>
allowDelegation?: boolean
isAnonymous?: boolean
    }
xTenantId: Shared_uuid
                        
                    };
VotingApiGet: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
VotingApiUpdate: {
                        authorization: string
id: Shared_uuid
requestBody: {
        title?: string
description?: string
startDate?: Shared_dateTime
endDate?: Shared_dateTime
    }
xTenantId: Shared_uuid
                        
                    };
VotingApiCancel: {
                        authorization: string
id: Shared_uuid
requestBody: {
        reason: string
    }
xTenantId: Shared_uuid
                        
                    };
VotingApiCastVote: {
                        authorization: string
id: Shared_uuid
requestBody: {
        selectedOptionIds: Array<Shared_uuid>
    }
xTenantId: Shared_uuid
                        
                    };
VotingApiGetMyBallot: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
VotingApiPublish: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
VotingApiGetResults: {
                        authorization: string
id: Shared_uuid
xTenantId: Shared_uuid
                        
                    };
        }
        
        
        responses: {
            VotingApiList: {
        /**
 * Array of items
 */
data: Array<Voting_Vote>
/**
 * Pagination metadata
 */
pagination: Shared_PaginationMeta
    }
                ,VotingApiCreate: Voting_Vote
                ,VotingApiGet: Voting_Vote
                ,VotingApiUpdate: Voting_Vote
                ,VotingApiCancel: Voting_Vote
                ,VotingApiCastVote: Voting_Ballot
                ,VotingApiGetMyBallot: Voting_Ballot
                ,VotingApiPublish: Voting_Vote
                ,VotingApiGetResults: Voting_VoteResult
                
        }
        
    }