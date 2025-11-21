"use client";

import { CircleCheck } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";

interface PricingFeature {
  text: string;
}

interface PricingPlan {
  id: string;
  name: string;
  description: string;
  price: string;
  features: PricingFeature[];
  button: {
    text: string;
    url: string;
  };
}

interface Pricing2Props {
  heading?: string;
  description?: string;
  plans?: PricingPlan[];
}

const Pricing2 = ({
  heading = "Pricing",
  description = "Check out our affordable pricing plans",
  plans = [
    {
      id: "plus",
      name: "Plus",
      description: "For personal use",
      price: "$19",
      features: [
        { text: "Up to 5 team members" },
        { text: "Basic components library" },
        { text: "Community support" },
        { text: "1GB storage space" },
      ],
      button: {
        text: "Purchase",
        url: "https://shadcnblocks.com",
      },
    },
    {
      id: "pro",
      name: "Pro",
      description: "For professionals",
      price: "$49",
      features: [
        { text: "Unlimited team members" },
        { text: "Advanced components" },
        { text: "Priority support" },
        { text: "Unlimited storage" },
      ],
      button: {
        text: "Purchase",
        url: "https://shadcnblocks.com",
      },
    },
  ],
}: Pricing2Props) => {
  return (
    <section className="w-full relative">
      <div className="mx-auto flex max-w-7xl flex-col items-center gap-12 text-center px-4 py-20 w-full">
        <div className="flex flex-col items-center gap-4">
          <h2 className="text-4xl font-bold lg:text-5xl text-primaryText">
            {heading}
          </h2>
          <p className="text-primaryText/70 text-lg max-w-2xl">{description}</p>
        </div>
        
        <div className="flex flex-col items-stretch gap-6 md:flex-row lg:gap-8 w-full justify-center pt-4">
          {plans.map((plan) => (
            <Card
              key={plan.id}
              className="flex w-full md:w-80 lg:w-96 flex-col justify-between text-left bg-muted/60 backdrop-blur-xl border border-white/10 hover:border-white/30 hover:shadow-2xl hover:shadow-bluePrimary/10 transition-all duration-300"
            >
              <CardHeader className="space-y-4">
                <div className="space-y-2">
                  <CardTitle className="text-2xl text-primaryText">
                    {plan.name}
                  </CardTitle>
                  <p className="text-primaryText/60 text-sm">
                    {plan.description}
                  </p>
                </div>
                <div className="flex items-end gap-1">
                  <span className="text-5xl font-bold text-primaryText">
                    {plan.price}
                  </span>
                  <span className="text-primaryText/60 text-lg font-semibold pb-2">
                    one-time
                  </span>
                </div>
              </CardHeader>
              <CardContent className="flex-1">
                <Separator className="mb-6 bg-white/10" />
                {plan.id === "pro" && (
                  <p className="mb-4 font-semibold text-greenSecondary text-sm">
                    Everything in Free, and:
                  </p>
                )}
                <ul className="space-y-3">
                  {plan.features.map((feature, index) => (
                    <li
                      key={index}
                      className="flex items-start gap-3 text-sm text-primaryText/80"
                    >
                      <CircleCheck className="size-5 text-greenSecondary flex-shrink-0 mt-0.5" />
                      <span>{feature.text}</span>
                    </li>
                  ))}
                </ul>
              </CardContent>
              <CardFooter className="mt-auto pt-6">
                <Button asChild className="w-full bg-bluePrimary hover:bg-bluePrimary/80 text-white h-11">
                  <a href={plan.button.url}>
                    {plan.button.text}
                  </a>
                </Button>
              </CardFooter>
            </Card>
          ))}
        </div>
      </div>
    </section>
  );
};

export { Pricing2 };
