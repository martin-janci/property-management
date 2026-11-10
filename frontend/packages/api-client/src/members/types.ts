/**
 * Organization members + unit residents — wire types.
 *
 * Mirrors api-server `routes/organizations/members.rs`,
 * `routes/organizations/roles.rs` and `routes/unit_residents.rs`
 * (snake_case JSON, no rename).
 */

/** `MemberResponse` */
export interface OrgMember {
  user_id: string;
  name: string;
  email: string;
  role_id?: string | null;
  role_name: string;
  role_type: string;
  joined_at?: string | null;
}

/** `ListMembersResponse` */
export interface OrgMembersResponse {
  members: OrgMember[];
  total: number;
}

/** `RoleResponse` */
export interface OrgRole {
  id: string;
  name: string;
  description?: string | null;
  permissions: string[];
  is_system: boolean;
}

/** `ListRolesResponse` */
export interface OrgRolesResponse {
  roles: OrgRole[];
  total: number;
}

/** `AddMemberRequest` — exactly one of `user_id` / `email`. */
export interface AddOrgMemberRequest {
  user_id?: string;
  email?: string;
  role_id: string;
  role_type?: string;
}

/** `UpdateMemberRequest` */
export interface UpdateOrgMemberRequest {
  role_id: string;
  role_type?: string;
}

export type ResidentType = 'owner' | 'tenant' | 'family_member' | 'subtenant';

export const RESIDENT_TYPES: readonly ResidentType[] = [
  'owner',
  'tenant',
  'family_member',
  'subtenant',
] as const;

/** `ResidentWithUserResponse` (list item) */
export interface UnitResident {
  id: string;
  unit_id: string;
  user_id: string;
  user_name: string;
  user_email: string;
  resident_type: ResidentType | string;
  is_primary: boolean;
  start_date: string;
  end_date?: string | null;
  is_active: boolean;
}

/** `AddResidentRequest` */
export interface AddUnitResidentRequest {
  user_id: string;
  resident_type: ResidentType;
  is_primary?: boolean;
  /** ISO date (YYYY-MM-DD); server defaults to today. */
  start_date?: string;
  receives_notifications?: boolean;
  receives_mail?: boolean;
  notes?: string;
}
