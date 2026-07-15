import { z } from 'zod';

export const ProductSchema = z.enum(['ppt', 'reality', 'reality-mobile']);

export const PlatformSchema = z.enum([
  'ppt-web',
  'reality-web',
  'mobile',
  'mobile-native',
  'ios-swiftui',
  'android-kmp',
]);

export const BuildStatusSchema = z.enum(['planned', 'in-progress', 'shipped', 'n/a']);
export const RedesignStatusSchema = z.enum(['not-started', 'in-progress', 'applied', 'n/a']);
export const ApiStatusSchema = z.enum(['stub', 'partial', 'complete', 'n/a']);

export const RelatedRelSchema = z.enum(['parent', 'child', 'action', 'sibling', 'web-counterpart']);
export const DiagramKindSchema = z.enum(['sequence', 'flow', 'state', 'class']);

export const ImplementationSchema = z.object({
  route: z.string().optional(),
  screen: z.string().optional(),
  component: z.string().optional(),
  buildStatus: BuildStatusSchema,
  redesignStatus: RedesignStatusSchema,
  apiStatus: ApiStatusSchema,
});

export const RelatedScreenSchema = z.object({
  id: z.string().regex(/^[a-z][a-z0-9-]*\/[a-z0-9-]+$/, {
    message: 'related screen id must match <product>/<slug>',
  }),
  rel: RelatedRelSchema,
});

export const DiagramRefSchema = z.object({
  ref: z.string().min(1),
  kind: DiagramKindSchema,
});

export const DesignSourceRefSchema = z
  .object({
    adapter: z.string().min(1),
    file: z.string().optional(),
    frame: z.string().min(1),
  })
  .passthrough();

export const IdSchema = z.string().regex(/^(ppt|reality|reality-mobile)\/[a-z0-9-]+$/, {
  message: 'id must match <product>/<slug> using kebab-case',
});

const IsoDateSchema = z.preprocess(
  (value) => {
    if (value instanceof Date) {
      // gray-matter (via js-yaml) auto-coerces unquoted ISO dates into Date.
      return value.toISOString().slice(0, 10);
    }
    return value;
  },
  z.string().regex(/^\d{4}-\d{2}-\d{2}$/, { message: 'must be YYYY-MM-DD' })
);

export const ScreenMapFrontmatterSchema = z
  .object({
    id: IdSchema,
    name: z.string().min(1),
    product: ProductSchema,
    sitemapRefs: z.record(PlatformSchema, z.string()).optional(),
    implementations: z
      .record(PlatformSchema, ImplementationSchema)
      .refine((impls) => Object.keys(impls).length >= 1, {
        message:
          'implementations must list at least one platform (use buildStatus/redesignStatus/apiStatus = "n/a" for out-of-scope platforms)',
      }),
    endpoints: z.array(z.string()).optional(),
    relatedScreens: z.array(RelatedScreenSchema).optional(),
    sharedComponents: z.array(z.string()).optional(),
    diagrams: z.array(DiagramRefSchema).optional(),
    useCases: z.array(z.string()).optional(),
    epics: z.array(z.string()).optional(),
    designSources: z.array(DesignSourceRefSchema).optional(),
    owner: z.string().optional(),
    lastReview: IsoDateSchema.optional(),
  })
  .superRefine((value, ctx) => {
    const [productPrefix] = value.id.split('/');
    if (productPrefix !== value.product) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['id'],
        message: `id prefix "${productPrefix}" does not match product "${value.product}"`,
      });
    }
  });

export type ScreenMapFrontmatterInput = z.input<typeof ScreenMapFrontmatterSchema>;
