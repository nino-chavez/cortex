# UX Conversion Standards

Framework: Clarity -> Trust -> Action

## 1. Visual Hierarchy
- One H1 per page/section — states the core value proposition
- Subheading clarifies or qualifies the H1
- One primary CTA per viewport visible above the fold
- Supporting proof (testimonials, stats, logos) below the CTA
- Size, weight, and contrast must establish clear reading order

## 2. Typography
- Modular type scale (e.g., 1.25 ratio: 16, 20, 25, 31, 39px)
- Body text minimum 16px, line-height 1.5-1.75
- Contrast ratio 4.5:1 minimum (body), 3:1 (large text)
- Max line length 60-75 characters
- Intentional spacing between type levels (margin > font-size difference)

## 3. Friction Removal
- Forms: maximum 5 visible fields per step; use progressive disclosure for more
- Smart defaults and autofill where possible
- Clear, descriptive labels (not placeholder-only)
- Inline validation with specific error messages
- Minimize checkout/signup steps (ideal: 1-2 steps)

## 4. Scanning Behavior
- F-pattern for text-heavy pages: key content in top-left, left-aligned
- Z-pattern for landing/marketing pages: logo top-left, CTA top-right, content bottom-left, CTA bottom-right
- Bullet points over paragraphs for feature lists
- Bold/highlight key phrases within body text
- Break content into scannable chunks with clear headings

## 5. Performance
- Prefer lightweight layouts (CSS Grid/Flexbox) over heavy frameworks
- Images: lazy-load below-fold, use modern formats (WebP/AVIF), size appropriately
- Aim for Largest Contentful Paint (LCP) < 2.5s
- Avoid layout shift (CLS < 0.1) — reserve space for dynamic content
- Critical CSS inlined; non-critical deferred

## 6. Trust Signals
- Real testimonials with names/photos/roles where possible
- Client/partner logos in a recognizable strip
- Transparent pricing (no hidden fees, clear tier comparison)
- Security indicators on forms handling sensitive data (SSL badge, privacy note)
- Real product visuals over stock imagery

## 7. Color Psychology (60-30-10 Rule)
- 60% dominant color (background, large surfaces) — sets tone
- 30% secondary color (cards, sections, supporting elements) — creates contrast
- 10% accent color (CTAs, alerts, key interactive elements) — draws action
- CTA color must be the highest-contrast element on screen
- Maintain consistent color meaning across the application

## 8. Purposeful CTAs
- Every section/screen must have a clear next action
- Action-oriented copy: "Start Free Trial", "Get Your Report", "Book a Demo"
- Avoid generic labels: "Submit", "Click Here", "Learn More"
- High contrast against surrounding content (use the 10% accent color)
- Adequate whitespace around CTAs (minimum 16px padding, 24px+ margin)

## 9. Mobile-First
- Thumb-friendly tap targets: minimum 48x48px with 8px+ spacing
- Primary actions in bottom-third of screen (thumb zone)
- Simplified navigation: hamburger or bottom nav bar (max 5 items)
- Touch-optimized form inputs (appropriate keyboard types, large fields)
- No hover-dependent functionality

## 10. Continuous Optimization (Post-Launch)
- Instrument analytics on all CTAs and conversion funnels
- Plan for A/B testing on headlines, CTA copy, and layouts
- Track scroll depth and engagement heatmaps on key pages
- Monitor Core Web Vitals (LCP, FID/INP, CLS) continuously
- Iterate design decisions based on data, not assumptions
