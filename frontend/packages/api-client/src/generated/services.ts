import type { CancelablePromise } from './core/CancelablePromise';
import { OpenAPI } from './core/OpenAPI';
import { request as __request } from './core/request';
import type { AuthenticationData, BuildingsData, ComplianceGdprData, DocumentsData, FaultsData, RealEstateListingsData, OrganizationsData, ShortTermRentalsData, UnitsData, VotingData } from './models';

export class AuthenticationService {

	/**
	 * Login with email and password
	 * @returns Auth_LoginResponse The request has succeeded.
	 * @throws ApiError
	 */
	public static authApiLogin(data: AuthenticationData['payloads']['AuthApiLogin']): CancelablePromise<AuthenticationData['responses']['AuthApiLogin']> {
		const {
                        
                        requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/login',
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Logout (invalidate tokens)
	 * @returns void There is no content to send for this request, but the headers may be useful. 
	 * @throws ApiError
	 */
	public static authApiLogout(data: AuthenticationData['payloads']['AuthApiLogout']): CancelablePromise<AuthenticationData['responses']['AuthApiLogout']> {
		const {
                        
                        authorization
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/logout',
			headers: {
				Authorization: authorization
			},
		});
	}

	/**
	 * Get current user profile
	 * @returns Auth_User The request has succeeded.
	 * @throws ApiError
	 */
	public static authApiGetMe(data: AuthenticationData['payloads']['AuthApiGetMe']): CancelablePromise<AuthenticationData['responses']['AuthApiGetMe']> {
		const {
                        
                        authorization
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/auth/me',
			headers: {
				Authorization: authorization
			},
			errors: {
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Update current user profile
	 * @returns Auth_User The request has succeeded.
	 * @throws ApiError
	 */
	public static authApiUpdateMe(data: AuthenticationData['payloads']['AuthApiUpdateMe']): CancelablePromise<AuthenticationData['responses']['AuthApiUpdateMe']> {
		const {
                        
                        authorization,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/auth/me',
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Disable 2FA
	 * @returns void There is no content to send for this request, but the headers may be useful. 
	 * @throws ApiError
	 */
	public static authApiDisable2Fa(data: AuthenticationData['payloads']['AuthApiDisable2Fa']): CancelablePromise<AuthenticationData['responses']['AuthApiDisable2Fa']> {
		const {
                        
                        authorization,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/me/2fa/disable',
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Enable 2FA
	 * @returns void There is no content to send for this request, but the headers may be useful. 
	 * @throws ApiError
	 */
	public static authApiEnable2Fa(data: AuthenticationData['payloads']['AuthApiEnable2Fa']): CancelablePromise<AuthenticationData['responses']['AuthApiEnable2Fa']> {
		const {
                        
                        authorization,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/me/2fa/enable',
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Setup 2FA
	 * @returns Auth_TwoFactorSetupResponse The request has succeeded.
	 * @throws ApiError
	 */
	public static authApiSetup2Fa(data: AuthenticationData['payloads']['AuthApiSetup2Fa']): CancelablePromise<AuthenticationData['responses']['AuthApiSetup2Fa']> {
		const {
                        
                        authorization
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/me/2fa/setup',
			headers: {
				Authorization: authorization
			},
			errors: {
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Change password
	 * @returns void There is no content to send for this request, but the headers may be useful. 
	 * @throws ApiError
	 */
	public static authApiChangePassword(data: AuthenticationData['payloads']['AuthApiChangePassword']): CancelablePromise<AuthenticationData['responses']['AuthApiChangePassword']> {
		const {
                        
                        authorization,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/me/password',
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Request password reset
	 * @returns any The request has been accepted for processing, but processing has not yet completed.
	 * @throws ApiError
	 */
	public static authApiRequestPasswordReset(data: AuthenticationData['payloads']['AuthApiRequestPasswordReset']): CancelablePromise<AuthenticationData['responses']['AuthApiRequestPasswordReset']> {
		const {
                        
                        requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/password-reset',
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * Refresh access token
	 * @returns Auth_LoginResponse The request has succeeded.
	 * @throws ApiError
	 */
	public static authApiRefresh(data: AuthenticationData['payloads']['AuthApiRefresh']): CancelablePromise<AuthenticationData['responses']['AuthApiRefresh']> {
		const {
                        
                        authorization
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/refresh',
			headers: {
				Authorization: authorization
			},
			errors: {
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Register new user account
	 * @returns Auth_User The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static authApiRegister(data: AuthenticationData['payloads']['AuthApiRegister']): CancelablePromise<AuthenticationData['responses']['AuthApiRegister']> {
		const {
                        
                        requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/auth/register',
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				409: `Conflict - Resource already exists or version mismatch`,
			},
		});
	}

}

export class BuildingsService {

	/**
	 * List buildings
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static buildingsApiList(data: BuildingsData['payloads']['BuildingsApiList']): CancelablePromise<BuildingsData['responses']['BuildingsApiList']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
status,
type
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/buildings',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, status, type
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
			},
		});
	}

	/**
	 * Create new building
	 * @returns Buildings_Building The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static buildingsApiCreate(data: BuildingsData['payloads']['BuildingsApiCreate']): CancelablePromise<BuildingsData['responses']['BuildingsApiCreate']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/buildings',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
			},
		});
	}

	/**
	 * Get building by ID
	 * @returns Buildings_Building The request has succeeded.
	 * @throws ApiError
	 */
	public static buildingsApiGet(data: BuildingsData['payloads']['BuildingsApiGet']): CancelablePromise<BuildingsData['responses']['BuildingsApiGet']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/buildings/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Update building
	 * @returns Buildings_Building The request has succeeded.
	 * @throws ApiError
	 */
	public static buildingsApiUpdate(data: BuildingsData['payloads']['BuildingsApiUpdate']): CancelablePromise<BuildingsData['responses']['BuildingsApiUpdate']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/buildings/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Delete building
	 * @returns void There is no content to send for this request, but the headers may be useful. 
	 * @throws ApiError
	 */
	public static buildingsApiDelete(data: BuildingsData['payloads']['BuildingsApiDelete']): CancelablePromise<BuildingsData['responses']['BuildingsApiDelete']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'DELETE',
			url: '/api/v1/buildings/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * List common areas
	 * @returns Buildings_CommonArea The request has succeeded.
	 * @throws ApiError
	 */
	public static buildingsApiListCommonAreas(data: BuildingsData['payloads']['BuildingsApiListCommonAreas']): CancelablePromise<BuildingsData['responses']['BuildingsApiListCommonAreas']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/buildings/{id}/common-areas',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Create common area
	 * @returns Buildings_CommonArea The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static buildingsApiCreateCommonArea(data: BuildingsData['payloads']['BuildingsApiCreateCommonArea']): CancelablePromise<BuildingsData['responses']['BuildingsApiCreateCommonArea']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/buildings/{id}/common-areas',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * List building documents
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static buildingsApiListDocuments(data: BuildingsData['payloads']['BuildingsApiListDocuments']): CancelablePromise<BuildingsData['responses']['BuildingsApiListDocuments']> {
		const {
                        
                        authorization,
xTenantId,
id,
page,
limit,
sortBy,
sortOrder,
category
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/buildings/{id}/documents',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, category
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Upload building document
	 * @returns Buildings_BuildingDocument The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static buildingsApiUploadDocument(data: BuildingsData['payloads']['BuildingsApiUploadDocument']): CancelablePromise<BuildingsData['responses']['BuildingsApiUploadDocument']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/buildings/{id}/documents',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * List floors in building
	 * @returns Buildings_Floor The request has succeeded.
	 * @throws ApiError
	 */
	public static buildingsApiListFloors(data: BuildingsData['payloads']['BuildingsApiListFloors']): CancelablePromise<BuildingsData['responses']['BuildingsApiListFloors']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/buildings/{id}/floors',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Create floor
	 * @returns Buildings_Floor The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static buildingsApiCreateFloor(data: BuildingsData['payloads']['BuildingsApiCreateFloor']): CancelablePromise<BuildingsData['responses']['BuildingsApiCreateFloor']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/buildings/{id}/floors',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

}

export class ComplianceGdprService {

	/**
	 * Get audit logs (admin)
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static complianceApiGetAuditLogs(data: ComplianceGdprData['payloads']['ComplianceApiGetAuditLogs']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiGetAuditLogs']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
userId,
action,
resourceType,
from,
to
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/compliance/audit-logs',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, userId, action, resourceType, from, to
			},
		});
	}

	/**
	 * Get current consents
	 * @returns Compliance_Consent The request has succeeded.
	 * @throws ApiError
	 */
	public static complianceApiGetConsents(data: ComplianceGdprData['payloads']['ComplianceApiGetConsents']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiGetConsents']> {
		const {
                        
                        authorization
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/compliance/consents',
			headers: {
				Authorization: authorization
			},
		});
	}

	/**
	 * Update consent
	 * @returns Compliance_Consent The request has succeeded.
	 * @throws ApiError
	 */
	public static complianceApiUpdateConsent(data: ComplianceGdprData['payloads']['ComplianceApiUpdateConsent']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiUpdateConsent']> {
		const {
                        
                        authorization,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/compliance/consents',
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * Request data deletion (GDPR)
	 * @returns Compliance_DataDeletionRequest The request has been accepted for processing, but processing has not yet completed.
	 * @throws ApiError
	 */
	public static complianceApiRequestDeletion(data: ComplianceGdprData['payloads']['ComplianceApiRequestDeletion']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiRequestDeletion']> {
		const {
                        
                        authorization,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/compliance/gdpr/delete',
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * Get deletion request status
	 * @returns Compliance_DataDeletionRequest The request has succeeded.
	 * @throws ApiError
	 */
	public static complianceApiGetDeletionStatus(data: ComplianceGdprData['payloads']['ComplianceApiGetDeletionStatus']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiGetDeletionStatus']> {
		const {
                        
                        authorization,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/compliance/gdpr/delete/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Cancel deletion request
	 * @returns Compliance_DataDeletionRequest The request has succeeded.
	 * @throws ApiError
	 */
	public static complianceApiCancelDeletion(data: ComplianceGdprData['payloads']['ComplianceApiCancelDeletion']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiCancelDeletion']> {
		const {
                        
                        authorization,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/compliance/gdpr/delete/{id}/cancel',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Request data export (GDPR)
	 * @returns Compliance_DataExportRequest The request has been accepted for processing, but processing has not yet completed.
	 * @throws ApiError
	 */
	public static complianceApiRequestExport(data: ComplianceGdprData['payloads']['ComplianceApiRequestExport']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiRequestExport']> {
		const {
                        
                        authorization,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/compliance/gdpr/export',
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * Get export request status
	 * @returns Compliance_DataExportRequest The request has succeeded.
	 * @throws ApiError
	 */
	public static complianceApiGetExportStatus(data: ComplianceGdprData['payloads']['ComplianceApiGetExportStatus']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiGetExportStatus']> {
		const {
                        
                        authorization,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/compliance/gdpr/export/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Download exported data
	 * @returns binary The request has succeeded.
	 * @throws ApiError
	 */
	public static complianceApiDownloadExport(data: ComplianceGdprData['payloads']['ComplianceApiDownloadExport']): CancelablePromise<ComplianceGdprData['responses']['ComplianceApiDownloadExport']> {
		const {
                        
                        authorization,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/compliance/gdpr/export/{id}/download',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

}

export class DocumentsService {

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static documentsApiList(data: DocumentsData['payloads']['DocumentsApiList']): CancelablePromise<DocumentsData['responses']['DocumentsApiList']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
q,
status,
from,
to,
buildingId,
category,
folderId
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/documents',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, q, status, from, to, buildingId, category, folderId
			},
		});
	}

	/**
	 * @returns Documents_Document The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static documentsApiUpload(data: DocumentsData['payloads']['DocumentsApiUpload']): CancelablePromise<DocumentsData['responses']['DocumentsApiUpload']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/documents',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Documents_Folder The request has succeeded.
	 * @throws ApiError
	 */
	public static documentsApiListFolders(data: DocumentsData['payloads']['DocumentsApiListFolders']): CancelablePromise<DocumentsData['responses']['DocumentsApiListFolders']> {
		const {
                        
                        authorization,
xTenantId,
parentId
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/documents/folders',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				parentId
			},
		});
	}

	/**
	 * @returns Documents_Folder The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static documentsApiCreateFolder(data: DocumentsData['payloads']['DocumentsApiCreateFolder']): CancelablePromise<DocumentsData['responses']['DocumentsApiCreateFolder']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/documents/folders',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Documents_Document The request has succeeded.
	 * @throws ApiError
	 */
	public static documentsApiGet(data: DocumentsData['payloads']['DocumentsApiGet']): CancelablePromise<DocumentsData['responses']['DocumentsApiGet']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/documents/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * @returns Documents_Document The request has succeeded.
	 * @throws ApiError
	 */
	public static documentsApiUpdate(data: DocumentsData['payloads']['DocumentsApiUpdate']): CancelablePromise<DocumentsData['responses']['DocumentsApiUpdate']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/documents/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns void There is no content to send for this request, but the headers may be useful. 
	 * @throws ApiError
	 */
	public static documentsApiDelete(data: DocumentsData['payloads']['DocumentsApiDelete']): CancelablePromise<DocumentsData['responses']['DocumentsApiDelete']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'DELETE',
			url: '/api/v1/documents/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns binary The request has succeeded.
	 * @throws ApiError
	 */
	public static documentsApiDownload(data: DocumentsData['payloads']['DocumentsApiDownload']): CancelablePromise<DocumentsData['responses']['DocumentsApiDownload']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/documents/{id}/download',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns Documents_Document The request has succeeded.
	 * @throws ApiError
	 */
	public static documentsApiUploadNewVersion(data: DocumentsData['payloads']['DocumentsApiUploadNewVersion']): CancelablePromise<DocumentsData['responses']['DocumentsApiUploadNewVersion']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/documents/{id}/new-version',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Documents_Document The request has succeeded.
	 * @throws ApiError
	 */
	public static documentsApiListVersions(data: DocumentsData['payloads']['DocumentsApiListVersions']): CancelablePromise<DocumentsData['responses']['DocumentsApiListVersions']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/documents/{id}/versions',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

}

export class FaultsService {

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static faultsApiList(data: FaultsData['payloads']['FaultsApiList']): CancelablePromise<FaultsData['responses']['FaultsApiList']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
buildingId,
status,
priority,
category
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/faults',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, buildingId, status, priority, category
			},
		});
	}

	/**
	 * @returns Faults_Fault The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static faultsApiCreate(data: FaultsData['payloads']['FaultsApiCreate']): CancelablePromise<FaultsData['responses']['FaultsApiCreate']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/faults',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Faults_Fault The request has succeeded.
	 * @throws ApiError
	 */
	public static faultsApiGet(data: FaultsData['payloads']['FaultsApiGet']): CancelablePromise<FaultsData['responses']['FaultsApiGet']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/faults/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * @returns Faults_Fault The request has succeeded.
	 * @throws ApiError
	 */
	public static faultsApiUpdate(data: FaultsData['payloads']['FaultsApiUpdate']): CancelablePromise<FaultsData['responses']['FaultsApiUpdate']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/faults/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Faults_FaultComment The request has succeeded.
	 * @throws ApiError
	 */
	public static faultsApiListComments(data: FaultsData['payloads']['FaultsApiListComments']): CancelablePromise<FaultsData['responses']['FaultsApiListComments']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/faults/{id}/comments',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns Faults_FaultComment The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static faultsApiAddComment(data: FaultsData['payloads']['FaultsApiAddComment']): CancelablePromise<FaultsData['responses']['FaultsApiAddComment']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/faults/{id}/comments',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Shared_Attachment The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static faultsApiUploadPhoto(data: FaultsData['payloads']['FaultsApiUploadPhoto']): CancelablePromise<FaultsData['responses']['FaultsApiUploadPhoto']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/faults/{id}/photos',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Faults_Fault The request has succeeded.
	 * @throws ApiError
	 */
	public static faultsApiResolve(data: FaultsData['payloads']['FaultsApiResolve']): CancelablePromise<FaultsData['responses']['FaultsApiResolve']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/faults/{id}/resolve',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

}

export class RealEstateListingsService {

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiList(data: RealEstateListingsData['payloads']['ListingsApiList']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiList']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
type,
status,
minPrice,
maxPrice
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/listings',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, type, status, minPrice, maxPrice
			},
		});
	}

	/**
	 * @returns Listings_Listing The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static listingsApiCreate(data: RealEstateListingsData['payloads']['ListingsApiCreate']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiCreate']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/listings',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Listings_Inquiry The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiUpdateInquiry(data: RealEstateListingsData['payloads']['ListingsApiUpdateInquiry']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiUpdateInquiry']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/listings/inquiries/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Listings_PortalConfig The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiListPortals(data: RealEstateListingsData['payloads']['ListingsApiListPortals']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiListPortals']> {
		const {
                        
                        authorization,
xTenantId
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/listings/portals',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns Listings_PortalConfig The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static listingsApiConfigurePortal(data: RealEstateListingsData['payloads']['ListingsApiConfigurePortal']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiConfigurePortal']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/listings/portals',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Listings_Listing The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiGet(data: RealEstateListingsData['payloads']['ListingsApiGet']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiGet']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/listings/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * @returns Listings_Listing The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiUpdate(data: RealEstateListingsData['payloads']['ListingsApiUpdate']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiUpdate']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/listings/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiListInquiries(data: RealEstateListingsData['payloads']['ListingsApiListInquiries']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiListInquiries']> {
		const {
                        
                        authorization,
xTenantId,
id,
page,
limit,
sortBy,
sortOrder
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/listings/{id}/inquiries',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder
			},
		});
	}

	/**
	 * @returns Listings_Inquiry The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static listingsApiCreateInquiry(data: RealEstateListingsData['payloads']['ListingsApiCreateInquiry']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiCreateInquiry']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/listings/{id}/inquiries',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Shared_Attachment The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static listingsApiUploadPhoto(data: RealEstateListingsData['payloads']['ListingsApiUploadPhoto']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiUploadPhoto']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/listings/{id}/photos',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Listings_Listing The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiPublish(data: RealEstateListingsData['payloads']['ListingsApiPublish']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiPublish']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/listings/{id}/publish',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns Listings_PortalListing The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiPublishToPortal(data: RealEstateListingsData['payloads']['ListingsApiPublishToPortal']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiPublishToPortal']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/listings/{id}/publish-to-portal',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Listings_Listing The request has succeeded.
	 * @throws ApiError
	 */
	public static listingsApiUnpublish(data: RealEstateListingsData['payloads']['ListingsApiUnpublish']): CancelablePromise<RealEstateListingsData['responses']['ListingsApiUnpublish']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/listings/{id}/unpublish',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

}

export class OrganizationsService {

	/**
	 * List organizations (Super Admin only)
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static organizationsApiList(data: OrganizationsData['payloads']['OrganizationsApiList']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiList']> {
		const {
                        
                        authorization,
page,
limit,
sortBy,
sortOrder,
status
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/organizations',
			headers: {
				Authorization: authorization
			},
			query: {
				page, limit, sortBy, sortOrder, status
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
			},
		});
	}

	/**
	 * Create new organization
	 * @returns Organizations_Organization The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static organizationsApiCreate(data: OrganizationsData['payloads']['OrganizationsApiCreate']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiCreate']> {
		const {
                        
                        authorization,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/organizations',
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
			},
		});
	}

	/**
	 * Switch current tenant context
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static organizationsApiSwitchTenant(data: OrganizationsData['payloads']['OrganizationsApiSwitchTenant']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiSwitchTenant']> {
		const {
                        
                        authorization,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/organizations/switch/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Get organization by ID
	 * @returns Organizations_Organization The request has succeeded.
	 * @throws ApiError
	 */
	public static organizationsApiGet(data: OrganizationsData['payloads']['OrganizationsApiGet']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiGet']> {
		const {
                        
                        authorization,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/organizations/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Update organization
	 * @returns Organizations_Organization The request has succeeded.
	 * @throws ApiError
	 */
	public static organizationsApiUpdate(data: OrganizationsData['payloads']['OrganizationsApiUpdate']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiUpdate']> {
		const {
                        
                        authorization,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/organizations/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				400: `Bad Request - Invalid input`,
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Delete organization
	 * @returns void There is no content to send for this request, but the headers may be useful. 
	 * @throws ApiError
	 */
	public static organizationsApiDelete(data: OrganizationsData['payloads']['OrganizationsApiDelete']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiDelete']> {
		const {
                        
                        authorization,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'DELETE',
			url: '/api/v1/organizations/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Update organization branding
	 * @returns Organizations_Organization The request has succeeded.
	 * @throws ApiError
	 */
	public static organizationsApiUpdateBranding(data: OrganizationsData['payloads']['OrganizationsApiUpdateBranding']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiUpdateBranding']> {
		const {
                        
                        authorization,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PUT',
			url: '/api/v1/organizations/{id}/branding',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * List organization members
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static organizationsApiListMembers(data: OrganizationsData['payloads']['OrganizationsApiListMembers']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiListMembers']> {
		const {
                        
                        authorization,
id,
page,
limit,
sortBy,
sortOrder
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/organizations/{id}/members',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			query: {
				page, limit, sortBy, sortOrder
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Invite member to organization
	 * @returns Organizations_OrganizationMember The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static organizationsApiInviteMember(data: OrganizationsData['payloads']['OrganizationsApiInviteMember']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiInviteMember']> {
		const {
                        
                        authorization,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/organizations/{id}/members/invite',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
				409: `Conflict - Resource already exists or version mismatch`,
			},
		});
	}

	/**
	 * Update member role
	 * @returns Organizations_OrganizationMember The request has succeeded.
	 * @throws ApiError
	 */
	public static organizationsApiUpdateMember(data: OrganizationsData['payloads']['OrganizationsApiUpdateMember']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiUpdateMember']> {
		const {
                        
                        authorization,
id,
userId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/organizations/{id}/members/{userId}',
			path: {
				id, userId
			},
			headers: {
				Authorization: authorization
			},
			body: requestBody,
			mediaType: 'application/json',
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Remove member from organization
	 * @returns void There is no content to send for this request, but the headers may be useful. 
	 * @throws ApiError
	 */
	public static organizationsApiRemoveMember(data: OrganizationsData['payloads']['OrganizationsApiRemoveMember']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiRemoveMember']> {
		const {
                        
                        authorization,
id,
userId
                    } = data;
		return __request(OpenAPI, {
			method: 'DELETE',
			url: '/api/v1/organizations/{id}/members/{userId}',
			path: {
				id, userId
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * Get organization statistics
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static organizationsApiGetStats(data: OrganizationsData['payloads']['OrganizationsApiGetStats']): CancelablePromise<OrganizationsData['responses']['OrganizationsApiGetStats']> {
		const {
                        
                        authorization,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/organizations/{id}/stats',
			path: {
				id
			},
			headers: {
				Authorization: authorization
			},
			errors: {
				401: `Unauthorized - Authentication required`,
				403: `Forbidden - Insufficient permissions`,
				404: `Not Found - Resource does not exist`,
			},
		});
	}

}

export class ShortTermRentalsService {

	/**
	 * @returns Rentals_PlatformConnection The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiListConnections(data: ShortTermRentalsData['payloads']['RentalsApiListConnections']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiListConnections']> {
		const {
                        
                        authorization,
xTenantId,
unitId
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/rentals/connections',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				unitId
			},
		});
	}

	/**
	 * @returns Rentals_PlatformConnection The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static rentalsApiCreateConnection(data: ShortTermRentalsData['payloads']['RentalsApiCreateConnection']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiCreateConnection']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/rentals/connections',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiListGuests(data: ShortTermRentalsData['payloads']['RentalsApiListGuests']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiListGuests']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
unitId
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/rentals/guests',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, unitId
			},
		});
	}

	/**
	 * @returns Rentals_GuestRegistration The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static rentalsApiRegisterGuest(data: ShortTermRentalsData['payloads']['RentalsApiRegisterGuest']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiRegisterGuest']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/rentals/guests',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Rentals_GuestRegistration The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiSubmitToAuthorities(data: ShortTermRentalsData['payloads']['RentalsApiSubmitToAuthorities']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiSubmitToAuthorities']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/rentals/guests/{id}/submit-to-authorities',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiListReservations(data: ShortTermRentalsData['payloads']['RentalsApiListReservations']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiListReservations']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
unitId,
status,
from,
to
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/rentals/reservations',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, unitId, status, from, to
			},
		});
	}

	/**
	 * @returns Rentals_Reservation The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static rentalsApiCreateReservation(data: ShortTermRentalsData['payloads']['RentalsApiCreateReservation']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiCreateReservation']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/rentals/reservations',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Rentals_Reservation The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiGetReservation(data: ShortTermRentalsData['payloads']['RentalsApiGetReservation']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiGetReservation']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/rentals/reservations/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * @returns Rentals_Reservation The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiCheckIn(data: ShortTermRentalsData['payloads']['RentalsApiCheckIn']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiCheckIn']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/rentals/reservations/{id}/check-in',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns Rentals_Reservation The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiCheckOut(data: ShortTermRentalsData['payloads']['RentalsApiCheckOut']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiCheckOut']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/rentals/reservations/{id}/check-out',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiGenerateAccessCode(data: ShortTermRentalsData['payloads']['RentalsApiGenerateAccessCode']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiGenerateAccessCode']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/rentals/reservations/{id}/generate-access-code',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static rentalsApiSyncPlatforms(data: ShortTermRentalsData['payloads']['RentalsApiSyncPlatforms']): CancelablePromise<ShortTermRentalsData['responses']['RentalsApiSyncPlatforms']> {
		const {
                        
                        authorization,
xTenantId,
unitId
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/rentals/sync',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				unitId
			},
		});
	}

}

export class UnitsService {

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static unitsApiList(data: UnitsData['payloads']['UnitsApiList']): CancelablePromise<UnitsData['responses']['UnitsApiList']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
buildingId,
status
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/units',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, buildingId, status
			},
		});
	}

	/**
	 * @returns Units_Unit The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static unitsApiCreate(data: UnitsData['payloads']['UnitsApiCreate']): CancelablePromise<UnitsData['responses']['UnitsApiCreate']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/units',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Units_Unit The request has succeeded.
	 * @throws ApiError
	 */
	public static unitsApiGet(data: UnitsData['payloads']['UnitsApiGet']): CancelablePromise<UnitsData['responses']['UnitsApiGet']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/units/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * @returns Units_Unit The request has succeeded.
	 * @throws ApiError
	 */
	public static unitsApiUpdate(data: UnitsData['payloads']['UnitsApiUpdate']): CancelablePromise<UnitsData['responses']['UnitsApiUpdate']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/units/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Units_UnitOwner The request has succeeded.
	 * @throws ApiError
	 */
	public static unitsApiListOwners(data: UnitsData['payloads']['UnitsApiListOwners']): CancelablePromise<UnitsData['responses']['UnitsApiListOwners']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/units/{id}/owners',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns Units_UnitOwner The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static unitsApiAddOwner(data: UnitsData['payloads']['UnitsApiAddOwner']): CancelablePromise<UnitsData['responses']['UnitsApiAddOwner']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/units/{id}/owners',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Units_UnitResident The request has succeeded.
	 * @throws ApiError
	 */
	public static unitsApiListResidents(data: UnitsData['payloads']['UnitsApiListResidents']): CancelablePromise<UnitsData['responses']['UnitsApiListResidents']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/units/{id}/residents',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns Units_UnitResident The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static unitsApiAddResident(data: UnitsData['payloads']['UnitsApiAddResident']): CancelablePromise<UnitsData['responses']['UnitsApiAddResident']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/units/{id}/residents',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

}

export class VotingService {

	/**
	 * @returns any The request has succeeded.
	 * @throws ApiError
	 */
	public static votingApiList(data: VotingData['payloads']['VotingApiList']): CancelablePromise<VotingData['responses']['VotingApiList']> {
		const {
                        
                        authorization,
xTenantId,
page,
limit,
sortBy,
sortOrder,
buildingId,
status
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/votes',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			query: {
				page, limit, sortBy, sortOrder, buildingId, status
			},
		});
	}

	/**
	 * @returns Voting_Vote The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static votingApiCreate(data: VotingData['payloads']['VotingApiCreate']): CancelablePromise<VotingData['responses']['VotingApiCreate']> {
		const {
                        
                        authorization,
xTenantId,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/votes',
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Voting_Vote The request has succeeded.
	 * @throws ApiError
	 */
	public static votingApiGet(data: VotingData['payloads']['VotingApiGet']): CancelablePromise<VotingData['responses']['VotingApiGet']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/votes/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * @returns Voting_Vote The request has succeeded.
	 * @throws ApiError
	 */
	public static votingApiUpdate(data: VotingData['payloads']['VotingApiUpdate']): CancelablePromise<VotingData['responses']['VotingApiUpdate']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'PATCH',
			url: '/api/v1/votes/{id}',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Voting_Vote The request has succeeded.
	 * @throws ApiError
	 */
	public static votingApiCancel(data: VotingData['payloads']['VotingApiCancel']): CancelablePromise<VotingData['responses']['VotingApiCancel']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/votes/{id}/cancel',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Voting_Ballot The request has succeeded and a new resource has been created as a result.
	 * @throws ApiError
	 */
	public static votingApiCastVote(data: VotingData['payloads']['VotingApiCastVote']): CancelablePromise<VotingData['responses']['VotingApiCastVote']> {
		const {
                        
                        authorization,
xTenantId,
id,
requestBody
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/votes/{id}/cast',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			body: requestBody,
			mediaType: 'application/json',
		});
	}

	/**
	 * @returns Voting_Ballot The request has succeeded.
	 * @throws ApiError
	 */
	public static votingApiGetMyBallot(data: VotingData['payloads']['VotingApiGetMyBallot']): CancelablePromise<VotingData['responses']['VotingApiGetMyBallot']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/votes/{id}/my-ballot',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

	/**
	 * @returns Voting_Vote The request has succeeded.
	 * @throws ApiError
	 */
	public static votingApiPublish(data: VotingData['payloads']['VotingApiPublish']): CancelablePromise<VotingData['responses']['VotingApiPublish']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'POST',
			url: '/api/v1/votes/{id}/publish',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
		});
	}

	/**
	 * @returns Voting_VoteResult The request has succeeded.
	 * @throws ApiError
	 */
	public static votingApiGetResults(data: VotingData['payloads']['VotingApiGetResults']): CancelablePromise<VotingData['responses']['VotingApiGetResults']> {
		const {
                        
                        authorization,
xTenantId,
id
                    } = data;
		return __request(OpenAPI, {
			method: 'GET',
			url: '/api/v1/votes/{id}/results',
			path: {
				id
			},
			headers: {
				Authorization: authorization, 'X-Tenant-ID': xTenantId
			},
			errors: {
				404: `Not Found - Resource does not exist`,
			},
		});
	}

}