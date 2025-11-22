import { ArrowRight } from "lucide-react";

import { Button } from "@/components/ui/button";

interface Feature {
  id: string;
  heading: string;
  description: string;
  image?: string;
  video?: string;
  url?: string;
}

interface Feature72Props {
  title: string;
  description?: string;
  buttonUrl?: string;
  buttonText?: string;
  features?: Feature[];
}

const Feature72 = ({
  title = "",
  description="",
  features = [],
}: Feature72Props) => {
  return (
    <section className="py-32">
      <div className="container w-full flex flex-col items-center">
        <div className="mb-12 flex flex-col items-center text-center max-w-3xl">
          <h2 className="mb-4 text-4xl font-bold text-primaryText md:text-5xl lg:text-6xl">
            {title}
          </h2>
          {description && (
            <p className="text-primaryText/70 text-lg lg:text-xl">
              {description}
            </p>
          )}
        </div>
        <div className="grid gap-6 md:grid-cols-2 lg:gap-8">
          {features.map((feature) => (
            <div
              key={feature.id}
              className="flex flex-col overflow-clip rounded-xl bg-muted/30 border-primaryText/20"
            >
              <div className="relative aspect-video w-full overflow-hidden bg-black flex items-center justify-center">
                {feature.video ? (
                  <video
                    src={feature.video}
                    className="h-full w-full object-fit"
                    autoPlay
                    loop
                    muted
                    playsInline
                  />
                ) : feature.image ? (
                  <img
                    src={feature.image}
                    alt={feature.heading}
                    className="h-full w-full object-cover object-center transition-opacity hover:opacity-80"
                  />
                ) : (
                  <div className="flex items-center justify-center h-full w-full">
                    <div className="text-primaryText/40 text-6xl font-bold">Coming Soon</div>
                  </div>
                )}
              </div>
              <div className="px-6 py-8 md:px-8 md:py-10 lg:px-10 lg:py-12">
                <h3 className="mb-3 text-lg font-semibold text-primaryText md:mb-4 md:text-2xl lg:mb-6">
                  {feature.heading}
                </h3>
                <p className="text-primaryText/70 lg:text-lg">
                  {feature.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};

export { Feature72 };
