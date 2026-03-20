

export const $Auth_LoginRequest = {
    type: 'object',
    required: ['email', 'password'],
    properties: {
        email: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.email'
                }
            ],
            description: 'Email address'
        },
        password: {
            type: 'string',
            format: 'password',
            description: 'Password'
        },
        twoFactorCode: {
            type: 'string',
            description: '2FA code (if enabled)'
        }
    },
    description: 'Login request'
} as const;

export const $Auth_LoginResponse = {
    type: 'object',
    required: ['accessToken', 'refreshToken', 'expiresIn', 'user', 'tenants'],
    properties: {
        accessToken: {
            type: 'string',
            description: 'Access token (JWT)'
        },
        refreshToken: {
            type: 'string',
            description: 'Refresh token'
        },
        expiresIn: {
            type: 'integer',
            format: 'int32',
            description: 'Token expiration in seconds'
        },
        user: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Auth.User'
                }
            ],
            description: 'User info'
        },
        tenants: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Auth.TenantMembership'
            },
            description: 'Available tenants for this user'
        }
    },
    description: 'Login response'
} as const;

export const $Auth_PasswordChangeRequest = {
    type: 'object',
    required: ['currentPassword', 'newPassword'],
    properties: {
        currentPassword: {
            type: 'string',
            format: 'password',
            description: 'Current password'
        },
        newPassword: {
            type: 'string',
            minLength: 8,
            format: 'password',
            description: 'New password'
        }
    },
    description: 'Password change request'
} as const;

export const $Auth_PasswordResetRequest = {
    type: 'object',
    required: ['email'],
    properties: {
        email: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.email'
                }
            ],
            description: 'Email address'
        }
    },
    description: 'Password reset request'
} as const;

export const $Auth_RegisterRequest = {
    type: 'object',
    required: ['email', 'password', 'displayName'],
    properties: {
        email: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.email'
                }
            ],
            description: 'Email address'
        },
        password: {
            type: 'string',
            minLength: 8,
            format: 'password',
            description: 'Password'
        },
        displayName: {
            type: 'string',
            description: 'Display name'
        },
        phone: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.phoneNumber'
                }
            ],
            description: 'Phone number'
        },
        invitationCode: {
            type: 'string',
            description: 'Invitation code (if required)'
        }
    },
    description: 'Registration request'
} as const;

export const $Auth_TenantMembership = {
    type: 'object',
    required: ['tenantId', 'tenantName', 'role'],
    properties: {
        tenantId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Tenant ID'
        },
        tenantName: {
            type: 'string',
            description: 'Tenant/Organization name'
        },
        role: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.TenantRole'
                }
            ],
            description: "User's role in this tenant"
        }
    },
    description: "User's membership in a tenant"
} as const;

export const $Auth_TwoFactorSetupResponse = {
    type: 'object',
    required: ['secret', 'qrCodeUrl', 'backupCodes'],
    properties: {
        secret: {
            type: 'string',
            description: 'Secret key for authenticator app'
        },
        qrCodeUrl: {
            type: 'string',
            description: 'QR code data URL'
        },
        backupCodes: {
            type: 'array',
            items: {
                type: 'string'
            },
            description: 'Backup codes'
        }
    },
    description: '2FA setup response'
} as const;

export const $Auth_User = {
    type: 'object',
    required: ['id', 'email', 'displayName', 'emailVerified', 'twoFactorEnabled', 'status', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Unique identifier'
        },
        email: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.email'
                }
            ],
            description: 'Email address'
        },
        displayName: {
            type: 'string',
            description: 'Display name'
        },
        phone: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.phoneNumber'
                }
            ],
            description: 'Phone number'
        },
        avatarUrl: {
            type: 'string',
            format: 'uri',
            description: 'Profile picture URL'
        },
        emailVerified: {
            type: 'boolean',
            description: 'Whether email is verified'
        },
        twoFactorEnabled: {
            type: 'boolean',
            description: 'Whether 2FA is enabled'
        },
        lastLoginAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last login timestamp'
        },
        status: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Auth.UserStatus'
                }
            ],
            description: 'Account status'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'User account'
} as const;

export const $Auth_UserStatus = {
    type: 'string',
    enum: ['active', 'inactive', 'suspended', 'pending_verification'],
    description: 'User account status'
} as const;

export const $Buildings_Building = {
    type: 'object',
    required: ['id', 'organizationId', 'name', 'address', 'type', 'floorCount', 'unitCount', 'status', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Unique identifier'
        },
        organizationId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Organization ID'
        },
        name: {
            type: 'string',
            description: 'Building name'
        },
        address: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.Address'
                }
            ],
            description: 'Building address'
        },
        location: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.GeoLocation'
                }
            ],
            description: 'GPS location'
        },
        type: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Buildings.BuildingType'
                }
            ],
            description: 'Building type'
        },
        floorCount: {
            type: 'integer',
            format: 'int32',
            description: 'Number of floors'
        },
        unitCount: {
            type: 'integer',
            format: 'int32',
            description: 'Number of units'
        },
        yearBuilt: {
            type: 'integer',
            format: 'int32',
            description: 'Year built'
        },
        totalAreaM2: {
            type: 'number',
            format: 'double',
            description: 'Total building area in m2'
        },
        photoUrl: {
            type: 'string',
            format: 'uri',
            description: 'Building photo URL'
        },
        description: {
            type: 'string',
            description: 'Building description'
        },
        status: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Buildings.BuildingStatus'
                }
            ],
            description: 'Building status'
        },
        managerId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Manager user ID'
        },
        technicalManagerId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Technical manager user ID'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Building'
} as const;

export const $Buildings_BuildingDocument = {
    type: 'object',
    required: ['id', 'buildingId', 'title', 'category', 'file', 'visibility', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Unique identifier'
        },
        buildingId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Building ID'
        },
        title: {
            type: 'string',
            description: 'Document title'
        },
        category: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Buildings.DocumentCategory'
                }
            ],
            description: 'Document category'
        },
        file: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.Attachment'
                }
            ],
            description: 'File attachment'
        },
        description: {
            type: 'string',
            description: 'Description'
        },
        visibility: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Buildings.DocumentVisibility'
                }
            ],
            description: 'Visibility'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Building document'
} as const;

export const $Buildings_BuildingStatus = {
    type: 'string',
    enum: ['active', 'under_construction', 'renovation', 'inactive'],
    description: 'Building status'
} as const;

export const $Buildings_BuildingType = {
    type: 'string',
    enum: ['residential', 'commercial', 'mixed', 'industrial'],
    description: 'Building type'
} as const;

export const $Buildings_CommonArea = {
    type: 'object',
    required: ['id', 'buildingId', 'name', 'type'],
    properties: {
        id: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Unique identifier'
        },
        buildingId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Building ID'
        },
        name: {
            type: 'string',
            description: 'Area name'
        },
        type: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Buildings.CommonAreaType'
                }
            ],
            description: 'Area type'
        },
        description: {
            type: 'string',
            description: 'Description'
        },
        areaM2: {
            type: 'number',
            format: 'double',
            description: 'Area size in m2'
        },
        floorId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Floor ID (if applicable)'
        },
        photoUrl: {
            type: 'string',
            format: 'uri',
            description: 'Photo URL'
        }
    },
    description: 'Common area (staircase, elevator, parking, garden, etc.)'
} as const;

export const $Buildings_CommonAreaType = {
    type: 'string',
    enum: ['staircase', 'elevator', 'lobby', 'hallway', 'basement', 'attic', 'parking', 'garage', 'garden', 'playground', 'pool', 'gym', 'laundry_room', 'storage_room', 'other'],
    description: 'Common area type'
} as const;

export const $Buildings_CreateBuildingRequest = {
    type: 'object',
    required: ['name', 'address', 'type', 'floorCount'],
    properties: {
        name: {
            type: 'string'
        },
        address: {
            '$ref': '#/components/schemas/Shared.Address'
        },
        location: {
            '$ref': '#/components/schemas/Shared.GeoLocation'
        },
        type: {
            '$ref': '#/components/schemas/Buildings.BuildingType'
        },
        floorCount: {
            type: 'integer',
            format: 'int32'
        },
        yearBuilt: {
            type: 'integer',
            format: 'int32'
        },
        totalAreaM2: {
            type: 'number',
            format: 'double'
        },
        description: {
            type: 'string'
        },
        managerId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        technicalManagerId: {
            '$ref': '#/components/schemas/Shared.uuid'
        }
    },
    description: 'Create building request'
} as const;

export const $Buildings_DocumentCategory = {
    type: 'string',
    enum: ['legal_document', 'insurance', 'maintenance', 'financial_report', 'meeting_minutes', 'contract', 'technical_document', 'other'],
    description: 'Document category'
} as const;

export const $Buildings_DocumentVisibility = {
    type: 'string',
    enum: ['public', 'owners_only', 'managers_only'],
    description: 'Document visibility'
} as const;

export const $Buildings_Floor = {
    type: 'object',
    required: ['id', 'buildingId', 'number', 'unitCount'],
    properties: {
        id: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Unique identifier'
        },
        buildingId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Building ID'
        },
        number: {
            type: 'integer',
            format: 'int32',
            description: 'Floor number (0 = ground, -1 = basement)'
        },
        name: {
            type: 'string',
            description: 'Floor name/label'
        },
        floorPlanUrl: {
            type: 'string',
            format: 'uri',
            description: 'Floor plan URL'
        },
        unitCount: {
            type: 'integer',
            format: 'int32',
            description: 'Number of units on this floor'
        }
    },
    description: 'Floor in a building'
} as const;

export const $Compliance_AuditLog = {
    type: 'object',
    required: ['id', 'organizationId', 'action', 'resourceType', 'timestamp'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        organizationId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        userId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        action: {
            type: 'string'
        },
        resourceType: {
            type: 'string'
        },
        resourceId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        oldValue: {
            type: 'string'
        },
        newValue: {
            type: 'string'
        },
        ipAddress: {
            type: 'string'
        },
        userAgent: {
            type: 'string'
        },
        timestamp: {
            '$ref': '#/components/schemas/Shared.dateTime'
        }
    },
    description: 'Audit log entry'
} as const;

export const $Compliance_Consent = {
    type: 'object',
    required: ['id', 'userId', 'consentType', 'granted', 'version'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        userId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        consentType: {
            '$ref': '#/components/schemas/Compliance.ConsentType'
        },
        granted: {
            type: 'boolean'
        },
        grantedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        revokedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        ipAddress: {
            type: 'string'
        },
        userAgent: {
            type: 'string'
        },
        version: {
            type: 'string'
        }
    },
    description: 'User consent record'
} as const;

export const $Compliance_ConsentType = {
    type: 'string',
    enum: ['terms_of_service', 'privacy_policy', 'marketing', 'analytics', 'third_party_sharing', 'cookies']
} as const;

export const $Compliance_DataCategory = {
    type: 'string',
    enum: ['profile', 'units', 'documents', 'messages', 'votes', 'faults', 'payments', 'audit_logs']
} as const;

export const $Compliance_DataDeletionRequest = {
    type: 'object',
    required: ['id', 'userId', 'status', 'requestedAt', 'scheduledFor', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        userId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        status: {
            '$ref': '#/components/schemas/Compliance.DeletionStatus'
        },
        reason: {
            type: 'string'
        },
        requestedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        scheduledFor: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        completedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        retainedDataCategories: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Compliance.DataCategory'
            }
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'GDPR data deletion request'
} as const;

export const $Compliance_DataExportRequest = {
    type: 'object',
    required: ['id', 'userId', 'status', 'format', 'requestedAt', 'includeCategories', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        userId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        status: {
            '$ref': '#/components/schemas/Compliance.ExportStatus'
        },
        format: {
            '$ref': '#/components/schemas/Compliance.ExportFormat'
        },
        requestedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        completedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        downloadUrl: {
            '$ref': '#/components/schemas/Shared.urlString'
        },
        expiresAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        includeCategories: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Compliance.DataCategory'
            }
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'GDPR data export request'
} as const;

export const $Compliance_DeletionStatus = {
    type: 'string',
    enum: ['pending', 'scheduled', 'processing', 'completed', 'cancelled']
} as const;

export const $Compliance_ExportFormat = {
    type: 'string',
    enum: ['json', 'csv', 'pdf']
} as const;

export const $Compliance_ExportStatus = {
    type: 'string',
    enum: ['pending', 'processing', 'completed', 'expired', 'failed']
} as const;

export const $Documents_Document = {
    type: 'object',
    required: ['id', 'organizationId', 'title', 'category', 'file', 'visibility', 'version', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        organizationId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        buildingId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        title: {
            type: 'string'
        },
        description: {
            type: 'string'
        },
        category: {
            '$ref': '#/components/schemas/Documents.DocumentCategory'
        },
        file: {
            '$ref': '#/components/schemas/Shared.Attachment'
        },
        visibility: {
            '$ref': '#/components/schemas/Documents.DocumentVisibility'
        },
        tags: {
            type: 'array',
            items: {
                type: 'string'
            }
        },
        expiresAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        version: {
            type: 'integer',
            format: 'int32'
        },
        previousVersionId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Document (UC-08)'
} as const;

export const $Documents_DocumentCategory = {
    type: 'string',
    enum: ['contract', 'invoice', 'receipt', 'report', 'minutes', 'policy', 'manual', 'certificate', 'permit', 'insurance', 'other']
} as const;

export const $Documents_DocumentVisibility = {
    type: 'string',
    enum: ['public', 'building_only', 'owners_only', 'managers_only', 'private']
} as const;

export const $Documents_Folder = {
    type: 'object',
    required: ['id', 'organizationId', 'name', 'visibility', 'documentCount', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        organizationId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        parentId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        name: {
            type: 'string'
        },
        description: {
            type: 'string'
        },
        visibility: {
            '$ref': '#/components/schemas/Documents.DocumentVisibility'
        },
        documentCount: {
            type: 'integer',
            format: 'int32'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Document folder'
} as const;

export const $Faults_Fault = {
    type: 'object',
    required: ['id', 'buildingId', 'reporterId', 'title', 'description', 'category', 'priority', 'status', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        buildingId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        commonAreaId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        reporterId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        title: {
            type: 'string'
        },
        description: {
            type: 'string'
        },
        category: {
            '$ref': '#/components/schemas/Faults.FaultCategory'
        },
        priority: {
            '$ref': '#/components/schemas/Faults.FaultPriority'
        },
        status: {
            '$ref': '#/components/schemas/Faults.FaultStatus'
        },
        photos: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.Attachment'
            }
        },
        assignedTo: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        estimatedCost: {
            '$ref': '#/components/schemas/Shared.Money'
        },
        actualCost: {
            '$ref': '#/components/schemas/Shared.Money'
        },
        scheduledDate: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        completedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        resolution: {
            type: 'string'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Fault report (UC-03)'
} as const;

export const $Faults_FaultCategory = {
    type: 'string',
    enum: ['plumbing', 'electrical', 'hvac', 'structural', 'elevator', 'security', 'cleaning', 'landscaping', 'other']
} as const;

export const $Faults_FaultComment = {
    type: 'object',
    required: ['id', 'faultId', 'authorId', 'authorName', 'content', 'isInternal', 'createdAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        faultId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        authorId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        authorName: {
            type: 'string'
        },
        content: {
            type: 'string'
        },
        attachments: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.Attachment'
            }
        },
        isInternal: {
            type: 'boolean'
        },
        createdAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        }
    },
    description: 'Fault comment'
} as const;

export const $Faults_FaultPriority = {
    type: 'string',
    enum: ['low', 'medium', 'high', 'critical']
} as const;

export const $Faults_FaultStatus = {
    type: 'string',
    enum: ['reported', 'acknowledged', 'in_progress', 'on_hold', 'resolved', 'closed', 'rejected']
} as const;

export const $Listings_Inquiry = {
    type: 'object',
    required: ['id', 'listingId', 'name', 'email', 'message', 'source', 'status', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        listingId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        name: {
            type: 'string'
        },
        email: {
            '$ref': '#/components/schemas/Shared.email'
        },
        phone: {
            '$ref': '#/components/schemas/Shared.phoneNumber'
        },
        message: {
            type: 'string'
        },
        source: {
            '$ref': '#/components/schemas/Listings.InquirySource'
        },
        status: {
            '$ref': '#/components/schemas/Listings.InquiryStatus'
        },
        scheduledViewingAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        notes: {
            type: 'string'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Inquiry from potential buyer/renter'
} as const;

export const $Listings_InquirySource = {
    type: 'string',
    enum: ['website', 'portal', 'phone', 'email', 'walk_in']
} as const;

export const $Listings_InquiryStatus = {
    type: 'string',
    enum: ['new', 'contacted', 'viewing_scheduled', 'viewing_completed', 'interested', 'not_interested', 'closed']
} as const;

export const $Listings_Listing = {
    type: 'object',
    required: ['id', 'unitId', 'type', 'status', 'title', 'description', 'price', 'priceType', 'features', 'photos', 'viewCount', 'inquiryCount', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        type: {
            '$ref': '#/components/schemas/Listings.ListingType'
        },
        status: {
            '$ref': '#/components/schemas/Listings.ListingStatus'
        },
        title: {
            type: 'string'
        },
        description: {
            type: 'string'
        },
        price: {
            '$ref': '#/components/schemas/Shared.Money'
        },
        priceType: {
            '$ref': '#/components/schemas/Listings.PriceType'
        },
        features: {
            '$ref': '#/components/schemas/Listings.ListingFeatures'
        },
        photos: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.Attachment'
            }
        },
        virtualTourUrl: {
            type: 'string',
            format: 'uri'
        },
        videoUrl: {
            type: 'string',
            format: 'uri'
        },
        agentId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        publishedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        expiresAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        viewCount: {
            type: 'integer',
            format: 'int32'
        },
        inquiryCount: {
            type: 'integer',
            format: 'int32'
        },
        portalListings: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Listings.PortalListing'
            }
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Property listing for sale/rent (UC-31-32)'
} as const;

export const $Listings_ListingFeatures = {
    type: 'object',
    required: ['areaM2'],
    properties: {
        areaM2: {
            type: 'number',
            format: 'double'
        },
        roomCount: {
            type: 'integer',
            format: 'int32'
        },
        bathroomCount: {
            type: 'integer',
            format: 'int32'
        },
        floorNumber: {
            type: 'integer',
            format: 'int32'
        },
        totalFloors: {
            type: 'integer',
            format: 'int32'
        },
        yearBuilt: {
            type: 'integer',
            format: 'int32'
        },
        hasBalcony: {
            type: 'boolean'
        },
        hasParking: {
            type: 'boolean'
        },
        hasElevator: {
            type: 'boolean'
        },
        hasFurniture: {
            type: 'boolean'
        },
        petFriendly: {
            type: 'boolean'
        },
        energyRating: {
            type: 'string'
        },
        heatingType: {
            type: 'string'
        },
        amenities: {
            type: 'array',
            items: {
                type: 'string'
            }
        }
    }
} as const;

export const $Listings_ListingStatus = {
    type: 'string',
    enum: ['draft', 'active', 'pending', 'sold', 'rented', 'expired', 'archived']
} as const;

export const $Listings_ListingType = {
    type: 'string',
    enum: ['sale', 'long_term_rent', 'short_term_rent']
} as const;

export const $Listings_PortalConfig = {
    type: 'object',
    required: ['id', 'organizationId', 'portalName', 'apiEndpoint', 'apiKey', 'isActive', 'autoPublish', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        organizationId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        portalName: {
            type: 'string'
        },
        apiEndpoint: {
            type: 'string',
            format: 'uri'
        },
        apiKey: {
            type: 'string'
        },
        isActive: {
            type: 'boolean'
        },
        autoPublish: {
            type: 'boolean'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Real estate portal configuration'
} as const;

export const $Listings_PortalListing = {
    type: 'object',
    required: ['portalId', 'portalName', 'externalId', 'status'],
    properties: {
        portalId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        portalName: {
            type: 'string'
        },
        externalId: {
            type: 'string'
        },
        externalUrl: {
            type: 'string',
            format: 'uri'
        },
        status: {
            '$ref': '#/components/schemas/Listings.PortalListingStatus'
        },
        lastSyncAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        }
    },
    description: 'Listing on external portal'
} as const;

export const $Listings_PortalListingStatus = {
    type: 'string',
    enum: ['published', 'pending', 'error', 'removed']
} as const;

export const $Listings_PriceType = {
    type: 'string',
    enum: ['fixed', 'negotiable', 'per_month', 'per_night']
} as const;

export const $Organizations_CreateOrganizationRequest = {
    type: 'object',
    required: ['name', 'address', 'email', 'type'],
    properties: {
        name: {
            type: 'string'
        },
        legalName: {
            type: 'string'
        },
        taxId: {
            type: 'string'
        },
        registrationNumber: {
            type: 'string'
        },
        address: {
            '$ref': '#/components/schemas/Shared.Address'
        },
        email: {
            '$ref': '#/components/schemas/Shared.email'
        },
        phone: {
            '$ref': '#/components/schemas/Shared.phoneNumber'
        },
        website: {
            type: 'string',
            format: 'uri'
        },
        type: {
            '$ref': '#/components/schemas/Organizations.OrganizationType'
        }
    },
    description: 'Create organization request'
} as const;

export const $Organizations_InviteMemberRequest = {
    type: 'object',
    required: ['email', 'role'],
    properties: {
        email: {
            '$ref': '#/components/schemas/Shared.email'
        },
        role: {
            '$ref': '#/components/schemas/Shared.TenantRole'
        },
        assignedBuildingIds: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.uuid'
            }
        },
        message: {
            type: 'string'
        }
    },
    description: 'Invite member request'
} as const;

export const $Organizations_MemberStatus = {
    type: 'string',
    enum: ['active', 'invited', 'suspended'],
    description: 'Member status'
} as const;

export const $Organizations_Organization = {
    type: 'object',
    required: ['id', 'name', 'address', 'email', 'type', 'subscriptionTier', 'status', 'buildingCount', 'unitCount', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Unique identifier'
        },
        name: {
            type: 'string',
            description: 'Organization name'
        },
        legalName: {
            type: 'string',
            description: 'Legal name'
        },
        taxId: {
            type: 'string',
            description: 'Tax/VAT ID'
        },
        registrationNumber: {
            type: 'string',
            description: 'Registration number'
        },
        address: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.Address'
                }
            ],
            description: 'Headquarters address'
        },
        email: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.email'
                }
            ],
            description: 'Contact email'
        },
        phone: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.phoneNumber'
                }
            ],
            description: 'Contact phone'
        },
        website: {
            type: 'string',
            format: 'uri',
            description: 'Website URL'
        },
        logoUrl: {
            type: 'string',
            format: 'uri',
            description: 'Logo URL'
        },
        type: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Organizations.OrganizationType'
                }
            ],
            description: 'Organization type'
        },
        subscriptionTier: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Organizations.SubscriptionTier'
                }
            ],
            description: 'Subscription tier'
        },
        status: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Organizations.OrganizationStatus'
                }
            ],
            description: 'Organization status'
        },
        branding: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Organizations.OrganizationBranding'
                }
            ],
            description: 'Custom branding settings'
        },
        buildingCount: {
            type: 'integer',
            format: 'int32',
            description: 'Number of buildings'
        },
        unitCount: {
            type: 'integer',
            format: 'int32',
            description: 'Number of units'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Organization (housing cooperative, property management company)'
} as const;

export const $Organizations_OrganizationBranding = {
    type: 'object',
    properties: {
        primaryColor: {
            type: 'string',
            description: 'Primary color (hex)'
        },
        secondaryColor: {
            type: 'string',
            description: 'Secondary color (hex)'
        },
        logoUrl: {
            type: 'string',
            format: 'uri',
            description: 'Custom logo URL'
        },
        faviconUrl: {
            type: 'string',
            format: 'uri',
            description: 'Custom favicon URL'
        },
        emailFooter: {
            type: 'string',
            description: 'Custom email footer'
        }
    },
    description: 'Organization branding settings'
} as const;

export const $Organizations_OrganizationMember = {
    type: 'object',
    required: ['userId', 'email', 'displayName', 'role', 'assignedBuildingIds', 'joinedAt', 'status'],
    properties: {
        userId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'User ID'
        },
        email: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.email'
                }
            ],
            description: 'User email'
        },
        displayName: {
            type: 'string',
            description: 'User display name'
        },
        role: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.TenantRole'
                }
            ],
            description: 'Role in organization'
        },
        assignedBuildingIds: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.uuid'
            },
            description: 'Assigned buildings (empty = all)'
        },
        joinedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Joined timestamp'
        },
        status: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Organizations.MemberStatus'
                }
            ],
            description: 'Membership status'
        }
    },
    description: 'Organization member'
} as const;

export const $Organizations_OrganizationStatus = {
    type: 'string',
    enum: ['active', 'trial', 'suspended', 'cancelled'],
    description: 'Organization status'
} as const;

export const $Organizations_OrganizationType = {
    type: 'string',
    enum: ['housing_cooperative', 'property_management', 'real_estate_agency', 'individual'],
    description: 'Organization type'
} as const;

export const $Organizations_SubscriptionTier = {
    type: 'string',
    enum: ['free', 'starter', 'professional', 'enterprise'],
    description: 'Subscription tier'
} as const;

export const $Rentals_DocumentType = {
    type: 'string',
    enum: ['passport', 'id_card', 'drivers_license', 'other']
} as const;

export const $Rentals_GuestRegistration = {
    type: 'object',
    required: ['id', 'unitId', 'firstName', 'lastName', 'dateOfBirth', 'nationality', 'documentType', 'documentNumber', 'arrivalDate', 'departureDate', 'submittedToAuthorities', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        reservationId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        firstName: {
            type: 'string'
        },
        lastName: {
            type: 'string'
        },
        dateOfBirth: {
            '$ref': '#/components/schemas/Shared.date'
        },
        nationality: {
            type: 'string'
        },
        documentType: {
            '$ref': '#/components/schemas/Rentals.DocumentType'
        },
        documentNumber: {
            type: 'string'
        },
        documentExpiry: {
            '$ref': '#/components/schemas/Shared.date'
        },
        documentScanUrl: {
            type: 'string',
            format: 'uri'
        },
        arrivalDate: {
            '$ref': '#/components/schemas/Shared.date'
        },
        departureDate: {
            '$ref': '#/components/schemas/Shared.date'
        },
        submittedToAuthorities: {
            type: 'boolean'
        },
        submittedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        authorityReference: {
            type: 'string'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Guest registration for police/authorities'
} as const;

export const $Rentals_PlatformConnection = {
    type: 'object',
    required: ['id', 'unitId', 'platform', 'isActive', 'syncStatus', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        platform: {
            '$ref': '#/components/schemas/Rentals.RentalPlatform'
        },
        externalListingId: {
            type: 'string'
        },
        apiKey: {
            type: 'string'
        },
        isActive: {
            type: 'boolean'
        },
        lastSyncAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        syncStatus: {
            '$ref': '#/components/schemas/Rentals.SyncStatus'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Platform connection settings'
} as const;

export const $Rentals_RentalPlatform = {
    type: 'string',
    enum: ['airbnb', 'booking', 'vrbo', 'direct', 'other']
} as const;

export const $Rentals_Reservation = {
    type: 'object',
    required: ['id', 'unitId', 'platform', 'externalId', 'guestName', 'guestCount', 'checkIn', 'checkOut', 'status', 'totalPrice', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        platform: {
            '$ref': '#/components/schemas/Rentals.RentalPlatform'
        },
        externalId: {
            type: 'string'
        },
        guestName: {
            type: 'string'
        },
        guestEmail: {
            '$ref': '#/components/schemas/Shared.email'
        },
        guestPhone: {
            '$ref': '#/components/schemas/Shared.phoneNumber'
        },
        guestCount: {
            type: 'integer',
            format: 'int32'
        },
        checkIn: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        checkOut: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        status: {
            '$ref': '#/components/schemas/Rentals.ReservationStatus'
        },
        totalPrice: {
            '$ref': '#/components/schemas/Shared.Money'
        },
        platformFee: {
            '$ref': '#/components/schemas/Shared.Money'
        },
        cleaningFee: {
            '$ref': '#/components/schemas/Shared.Money'
        },
        accessCode: {
            type: 'string'
        },
        notes: {
            type: 'string'
        },
        guestRegistrationId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Short-term rental reservation (UC-29-30)'
} as const;

export const $Rentals_ReservationStatus = {
    type: 'string',
    enum: ['pending', 'confirmed', 'checked_in', 'checked_out', 'cancelled', 'no_show']
} as const;

export const $Rentals_SyncStatus = {
    type: 'string',
    enum: ['synced', 'pending', 'error', 'disabled']
} as const;

export const $Shared_Address = {
    type: 'object',
    required: ['street1', 'city', 'postalCode', 'country'],
    properties: {
        street1: {
            type: 'string',
            description: 'Street address line 1'
        },
        street2: {
            type: 'string',
            description: 'Street address line 2 (optional)'
        },
        city: {
            type: 'string',
            description: 'City name'
        },
        state: {
            type: 'string',
            description: 'State or province'
        },
        postalCode: {
            type: 'string',
            description: 'Postal/ZIP code'
        },
        country: {
            type: 'string',
            minLength: 2,
            maxLength: 2,
            description: 'ISO 3166-1 alpha-2 country code'
        }
    },
    description: 'Physical address'
} as const;

export const $Shared_Attachment = {
    type: 'object',
    required: ['id', 'filename', 'mimeType', 'sizeBytes', 'downloadUrl', 'uploadedAt'],
    properties: {
        id: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Unique identifier'
        },
        filename: {
            type: 'string',
            description: 'Original filename'
        },
        mimeType: {
            type: 'string',
            description: 'MIME type'
        },
        sizeBytes: {
            type: 'integer',
            format: 'int64',
            description: 'File size in bytes'
        },
        downloadUrl: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.urlString'
                }
            ],
            description: 'Download URL'
        },
        uploadedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Upload timestamp'
        }
    },
    description: 'File attachment metadata'
} as const;

export const $Shared_AuditableEntity = {
    type: 'object',
    required: ['createdAt', 'updatedAt'],
    properties: {
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Base model for all entities with audit fields'
} as const;

export const $Shared_AuthenticatedUser = {
    type: 'object',
    required: ['userId', 'tenantId', 'role', 'email', 'displayName'],
    properties: {
        userId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'User ID'
        },
        tenantId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Current tenant ID'
        },
        role: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.TenantRole'
                }
            ],
            description: "User's role in current tenant"
        },
        email: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.email'
                }
            ],
            description: "User's email"
        },
        displayName: {
            type: 'string',
            description: "User's display name"
        },
        delegations: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.Delegation'
            },
            description: 'Active delegations to this user'
        }
    },
    description: 'Authenticated user info (from JWT)'
} as const;

export const $Shared_CursorPaginationMeta = {
    type: 'object',
    required: ['hasMore'],
    properties: {
        nextCursor: {
            type: 'string',
            description: 'Cursor for next page (null if no more items)'
        },
        hasMore: {
            type: 'boolean',
            description: 'Has more items'
        }
    },
    description: 'Cursor-based pagination metadata'
} as const;

export const $Shared_CursorPaginationQuery = {
    type: 'object',
    properties: {
        cursor: {
            type: 'string',
            description: 'Cursor for next page'
        },
        limit: {
            type: 'integer',
            format: 'int32',
            minimum: 1,
            maximum: 100,
            description: 'Items per page',
            default: 20
        }
    },
    description: 'Cursor-based pagination query'
} as const;

export const $Shared_Delegation = {
    type: 'object',
    required: ['id', 'fromUserId', 'toUserId', 'accesses', 'startsAt', 'isActive'],
    properties: {
        id: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Unique identifier'
        },
        fromUserId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'User granting the delegation'
        },
        toUserId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'User receiving the delegation'
        },
        accesses: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.ResourceAccess'
            },
            description: 'Delegated resource accesses'
        },
        startsAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Delegation start date'
        },
        endsAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Delegation end date (null for permanent)'
        },
        isActive: {
            type: 'boolean',
            description: 'Whether delegation is currently active'
        },
        reason: {
            type: 'string',
            description: 'Reason for delegation'
        }
    },
    description: 'Delegation of rights'
} as const;

export const $Shared_ErrorResponse = {
    type: 'object',
    required: ['code', 'message', 'timestamp'],
    properties: {
        code: {
            type: 'string',
            description: 'Error code for programmatic handling'
        },
        message: {
            type: 'string',
            description: 'Human-readable error message'
        },
        requestId: {
            type: 'string',
            description: 'Request ID for support/debugging'
        },
        details: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.ValidationError'
            },
            description: 'Detailed validation errors'
        },
        timestamp: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'ISO 8601 timestamp'
        }
    },
    description: 'Standard error response'
} as const;

export const $Shared_GeoLocation = {
    type: 'object',
    required: ['latitude', 'longitude'],
    properties: {
        latitude: {
            type: 'number',
            format: 'double',
            minimum: -90,
            maximum: 90,
            description: 'Latitude (-90 to 90)'
        },
        longitude: {
            type: 'number',
            format: 'double',
            minimum: -180,
            maximum: 180,
            description: 'Longitude (-180 to 180)'
        }
    },
    description: 'GPS coordinates'
} as const;

export const $Shared_InternalServerError = {
    type: 'object',
    required: ['body'],
    properties: {
        body: {
            '$ref': '#/components/schemas/Shared.ErrorResponse'
        }
    },
    description: 'Internal Server Error'
} as const;

export const $Shared_LocalizedString = {
    type: 'object',
    required: ['default'],
    properties: {
        default: {
            type: 'string',
            description: 'Default language value'
        },
        translations: {
            type: 'object',
            additionalProperties: {
                type: 'string'
            },
            description: 'Translations by ISO 639-1 language code'
        }
    },
    description: 'Localized content'
} as const;

export const $Shared_Money = {
    type: 'object',
    required: ['amount', 'currency'],
    properties: {
        amount: {
            type: 'integer',
            format: 'int64',
            description: 'Amount in smallest currency unit (e.g., cents)'
        },
        currency: {
            type: 'string',
            minLength: 3,
            maxLength: 3,
            description: 'ISO 4217 currency code'
        }
    },
    description: 'Monetary amount with currency'
} as const;

export const $Shared_PaginationMeta = {
    type: 'object',
    required: ['page', 'limit', 'totalItems', 'totalPages', 'hasNext', 'hasPrevious'],
    properties: {
        page: {
            type: 'integer',
            format: 'int32',
            description: 'Current page number'
        },
        limit: {
            type: 'integer',
            format: 'int32',
            description: 'Items per page'
        },
        totalItems: {
            type: 'integer',
            format: 'int64',
            description: 'Total number of items'
        },
        totalPages: {
            type: 'integer',
            format: 'int32',
            description: 'Total number of pages'
        },
        hasNext: {
            type: 'boolean',
            description: 'Has next page'
        },
        hasPrevious: {
            type: 'boolean',
            description: 'Has previous page'
        }
    },
    description: 'Pagination metadata'
} as const;

export const $Shared_PermissionScope = {
    type: 'string',
    enum: ['read', 'create', 'update', 'delete', 'admin'],
    description: 'Permission scope'
} as const;

export const $Shared_RateLimitError = {
    type: 'object',
    required: ['body'],
    properties: {
        body: {
            '$ref': '#/components/schemas/Shared.ErrorResponse'
        }
    },
    description: 'Too Many Requests - Rate limit exceeded'
} as const;

export const $Shared_ResourceAccess = {
    type: 'object',
    required: ['resourceType', 'permissions'],
    properties: {
        resourceType: {
            type: 'string',
            description: "Resource type (e.g., 'building', 'unit', 'document')"
        },
        resourceId: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'Resource ID (null for all resources of type)'
        },
        permissions: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.PermissionScope'
            },
            description: 'Granted permissions'
        },
        expiresAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Access expiration (null for permanent)'
        }
    },
    description: 'Resource access level'
} as const;

export const $Shared_TenantContext = {
    type: 'object',
    description: 'Tenant context - required for all multi-tenant operations'
} as const;

export const $Shared_TenantRole = {
    type: 'string',
    enum: ['super_admin', 'org_admin', 'manager', 'technical_manager', 'owner', 'owner_delegate', 'tenant', 'resident', 'property_manager', 'real_estate_agent', 'guest'],
    description: 'User role within tenant'
} as const;

export const $Shared_UnprocessableEntityError = {
    type: 'object',
    required: ['body'],
    properties: {
        body: {
            '$ref': '#/components/schemas/Shared.ErrorResponse'
        }
    },
    description: 'Unprocessable Entity - Business rule violation'
} as const;

export const $Shared_ValidationError = {
    type: 'object',
    required: ['field', 'message', 'code'],
    properties: {
        field: {
            type: 'string',
            description: "Field path (e.g., 'address.city')"
        },
        message: {
            type: 'string',
            description: 'Error message for this field'
        },
        code: {
            type: 'string',
            description: 'Error code'
        }
    },
    description: 'Validation error detail'
} as const;

export const $Shared_date = {
    type: 'string',
    description: 'ISO 8601 date string (YYYY-MM-DD)'
} as const;

export const $Shared_dateTime = {
    type: 'string',
    description: 'ISO 8601 date-time string'
} as const;

export const $Shared_email = {
    type: 'string',
    format: 'email',
    description: 'Email address'
} as const;

export const $Shared_phoneNumber = {
    type: 'string',
    description: 'Phone number in E.164 format'
} as const;

export const $Shared_urlString = {
    type: 'string',
    format: 'uri',
    description: 'URL string'
} as const;

export const $Shared_uuid = {
    type: 'string',
    description: 'Unique identifier (UUID v4)'
} as const;

export const $Units_ResidentType = {
    type: 'string',
    enum: ['owner', 'tenant', 'family_member', 'subtenant']
} as const;

export const $Units_Unit = {
    type: 'object',
    required: ['id', 'buildingId', 'unitNumber', 'type', 'areaM2', 'hasParking', 'hasStorage', 'status', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        buildingId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        floorId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitNumber: {
            type: 'string'
        },
        type: {
            '$ref': '#/components/schemas/Units.UnitType'
        },
        areaM2: {
            type: 'number',
            format: 'double'
        },
        roomCount: {
            type: 'integer',
            format: 'int32'
        },
        bathroomCount: {
            type: 'integer',
            format: 'int32'
        },
        balconyCount: {
            type: 'integer',
            format: 'int32'
        },
        hasParking: {
            type: 'boolean'
        },
        hasStorage: {
            type: 'boolean'
        },
        status: {
            '$ref': '#/components/schemas/Units.UnitStatus'
        },
        ownershipShare: {
            type: 'number',
            format: 'double'
        },
        description: {
            type: 'string'
        },
        photoUrls: {
            type: 'array',
            items: {
                type: 'string',
                format: 'uri'
            }
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Property unit (apartment, office, etc.)'
} as const;

export const $Units_UnitOwner = {
    type: 'object',
    required: ['id', 'unitId', 'name', 'ownershipPercentage', 'isPrimaryOwner', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        userId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        name: {
            type: 'string'
        },
        email: {
            '$ref': '#/components/schemas/Shared.email'
        },
        phone: {
            '$ref': '#/components/schemas/Shared.phoneNumber'
        },
        ownershipPercentage: {
            type: 'number',
            format: 'double'
        },
        isPrimaryOwner: {
            type: 'boolean'
        },
        acquiredAt: {
            '$ref': '#/components/schemas/Shared.date'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Unit owner'
} as const;

export const $Units_UnitResident = {
    type: 'object',
    required: ['id', 'unitId', 'name', 'type', 'isActive', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        unitId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        userId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        name: {
            type: 'string'
        },
        email: {
            '$ref': '#/components/schemas/Shared.email'
        },
        phone: {
            '$ref': '#/components/schemas/Shared.phoneNumber'
        },
        type: {
            '$ref': '#/components/schemas/Units.ResidentType'
        },
        moveInDate: {
            '$ref': '#/components/schemas/Shared.date'
        },
        moveOutDate: {
            '$ref': '#/components/schemas/Shared.date'
        },
        isActive: {
            type: 'boolean'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Unit resident'
} as const;

export const $Units_UnitStatus = {
    type: 'string',
    enum: ['occupied', 'vacant', 'for_sale', 'for_rent', 'renovation']
} as const;

export const $Units_UnitType = {
    type: 'string',
    enum: ['apartment', 'studio', 'penthouse', 'office', 'retail', 'storage', 'parking']
} as const;

export const $Voting_Ballot = {
    type: 'object',
    required: ['id', 'voteId', 'voterId', 'selectedOptionIds', 'weight', 'submittedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        voteId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        voterId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        delegatedFrom: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        selectedOptionIds: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Shared.uuid'
            }
        },
        weight: {
            type: 'number',
            format: 'double'
        },
        submittedAt: {
            '$ref': '#/components/schemas/Shared.dateTime'
        }
    }
} as const;

export const $Voting_Vote = {
    type: 'object',
    required: ['id', 'buildingId', 'title', 'description', 'type', 'status', 'startDate', 'endDate', 'quorumPercentage', 'options', 'allowDelegation', 'isAnonymous', 'createdAt', 'updatedAt'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        buildingId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        title: {
            type: 'string'
        },
        description: {
            type: 'string'
        },
        type: {
            '$ref': '#/components/schemas/Voting.VoteType'
        },
        status: {
            '$ref': '#/components/schemas/Voting.VoteStatus'
        },
        startDate: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        endDate: {
            '$ref': '#/components/schemas/Shared.dateTime'
        },
        quorumPercentage: {
            type: 'number',
            format: 'double'
        },
        options: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Voting.VoteOption'
            }
        },
        allowDelegation: {
            type: 'boolean'
        },
        isAnonymous: {
            type: 'boolean'
        },
        createdAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Creation timestamp'
        },
        updatedAt: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.dateTime'
                }
            ],
            description: 'Last update timestamp'
        },
        createdBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who created this entity'
        },
        updatedBy: {
            allOf: [
                {
                    '$ref': '#/components/schemas/Shared.uuid'
                }
            ],
            description: 'ID of user who last updated this entity'
        }
    },
    description: 'Vote/Poll (UC-04)'
} as const;

export const $Voting_VoteOption = {
    type: 'object',
    required: ['id', 'text', 'order'],
    properties: {
        id: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        text: {
            type: 'string'
        },
        description: {
            type: 'string'
        },
        order: {
            type: 'integer',
            format: 'int32'
        }
    }
} as const;

export const $Voting_VoteOptionResult = {
    type: 'object',
    required: ['optionId', 'text', 'voteCount', 'weightedVotes', 'percentage'],
    properties: {
        optionId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        text: {
            type: 'string'
        },
        voteCount: {
            type: 'integer',
            format: 'int32'
        },
        weightedVotes: {
            type: 'number',
            format: 'double'
        },
        percentage: {
            type: 'number',
            format: 'double'
        }
    }
} as const;

export const $Voting_VoteResult = {
    type: 'object',
    required: ['voteId', 'totalEligibleVoters', 'totalVotes', 'participationRate', 'quorumReached', 'optionResults'],
    properties: {
        voteId: {
            '$ref': '#/components/schemas/Shared.uuid'
        },
        totalEligibleVoters: {
            type: 'integer',
            format: 'int32'
        },
        totalVotes: {
            type: 'integer',
            format: 'int32'
        },
        participationRate: {
            type: 'number',
            format: 'double'
        },
        quorumReached: {
            type: 'boolean'
        },
        optionResults: {
            type: 'array',
            items: {
                '$ref': '#/components/schemas/Voting.VoteOptionResult'
            }
        },
        winningOptionId: {
            '$ref': '#/components/schemas/Shared.uuid'
        }
    }
} as const;

export const $Voting_VoteStatus = {
    type: 'string',
    enum: ['draft', 'scheduled', 'active', 'closed', 'cancelled']
} as const;

export const $Voting_VoteType = {
    type: 'string',
    enum: ['simple', 'weighted', 'ranked_choice']
} as const;

export const $Shared_PaginationQuery_limit = {
    name: 'limit',
    in: 'query',
    required: false,
    description: 'Items per page',
    schema: {
        type: 'integer',
        format: 'int32',
        minimum: 1,
        maximum: 100,
        default: 20
    },
    explode: false
} as const;

export const $Shared_PaginationQuery_page = {
    name: 'page',
    in: 'query',
    required: false,
    description: 'Page number (1-indexed)',
    schema: {
        type: 'integer',
        format: 'int32',
        minimum: 1,
        default: 1
    },
    explode: false
} as const;

export const $Shared_PaginationQuery_sortBy = {
    name: 'sortBy',
    in: 'query',
    required: false,
    description: 'Sort field',
    schema: {
        type: 'string'
    },
    explode: false
} as const;

export const $Shared_PaginationQuery_sortOrder = {
    name: 'sortOrder',
    in: 'query',
    required: false,
    description: 'Sort direction',
    schema: {
        type: 'string',
        enum: ['asc', 'desc'],
        default: 'asc'
    },
    explode: false
} as const;

export const $Shared_SearchQuery_from = {
    name: 'from',
    in: 'query',
    required: false,
    description: 'Filter by date from (ISO 8601)',
    schema: {
        '$ref': '#/components/schemas/Shared.dateTime'
    },
    explode: false
} as const;

export const $Shared_SearchQuery_q = {
    name: 'q',
    in: 'query',
    required: false,
    description: 'Search term',
    schema: {
        type: 'string'
    },
    explode: false
} as const;

export const $Shared_SearchQuery_status = {
    name: 'status',
    in: 'query',
    required: false,
    description: 'Filter by status',
    schema: {
        type: 'string'
    },
    explode: false
} as const;

export const $Shared_SearchQuery_to = {
    name: 'to',
    in: 'query',
    required: false,
    description: 'Filter by date to (ISO 8601)',
    schema: {
        '$ref': '#/components/schemas/Shared.dateTime'
    },
    explode: false
} as const;