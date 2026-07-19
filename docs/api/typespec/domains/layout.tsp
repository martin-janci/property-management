import "../shared/models.tsp";
import "../shared/errors.tsp";

namespace PropertyManagement.Layout;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Which platform's component registry to resolve against.")
enum LayoutPlatform {
  web,
  mobile,
}

@doc("How a resolved section renders. Required sections that are hidden or killed degrade to placeholder; optional ones are omitted entirely.")
enum SectionPresentation {
  visible,
  placeholder,
}

@doc("One resolved section, ready to render.")
model ResolvedSection {
  @doc("Versioned semantic component type, e.g. price-box.v1")
  type: string;

  @doc("Display mode, guaranteed to be within the component's supported modes.")
  mode?: string;

  @doc("Component props (empty for placeholders).")
  props?: Record<unknown>;

  presentation: SectionPresentation;
}

@doc("A fully resolved screen layout: base config merged with platform, tenant and kill layers.")
model ResolvedScreen {
  @doc("Screen id, e.g. ppt/dashboard")
  screen: string;

  @doc("Published config version this resolution was computed from.")
  version: int32;

  sections: ResolvedSection[];
}

@doc("Sparse per-organization layout override (validated against superadmin-authored rails).")
model TenantLayoutOverride {
  @doc("Full desired section order by type; omitted = keep base order.")
  order?: string[];

  @doc("Per-section patches: visible / mode / whitelisted props.")
  sections?: Record<unknown>;
}

model TenantOverrideEnvelope {
  override: Record<unknown> | null;
  rails: Record<unknown>;
  published: Record<unknown> | null;
}

model PutTenantOverrideRequest {
  screen: string;
  override_config: TenantLayoutOverride;
}

@route("/api/v1/layout")
@tag("Layout")
interface LayoutApi {
  @doc("Resolve a screen's layout for the caller's organization and platform.")
  @get
  @route("/resolved/{screen}")
  getResolved(
    @path screen: string,
    @query platform?: LayoutPlatform,
  ): ResolvedScreen | NotFoundError | UnprocessableEntityError;

  @doc("Read the caller organization's override plus the rails and published base.")
  @get
  @route("/tenant-override")
  getTenantOverride(@query screen: string): TenantOverrideEnvelope | NotFoundError;

  @doc("Save the caller organization's override. Rejected with 422 when it exceeds the rails.")
  @put
  @route("/tenant-override")
  putTenantOverride(@body body: PutTenantOverrideRequest): TenantOverrideEnvelope | UnprocessableEntityError;
}
