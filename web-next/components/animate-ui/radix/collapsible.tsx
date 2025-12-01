'use client';

import * as React from 'react';
import { Collapsible as CollapsiblePrimitive } from 'radix-ui';

type CollapsibleContextType = {
  isOpen: boolean;
};

const CollapsibleContext = React.createContext<
  CollapsibleContextType | undefined
>(undefined);

const useCollapsible = (): CollapsibleContextType => {
  const context = React.useContext(CollapsibleContext);
  if (!context) {
    throw new Error('useCollapsible must be used within a Collapsible');
  }
  return context;
};

type CollapsibleProps = React.ComponentProps<typeof CollapsiblePrimitive.Root>;

function Collapsible({ children, ...props }: CollapsibleProps) {
  const [isOpen, setIsOpen] = React.useState(
    props?.open ?? props?.defaultOpen ?? false,
  );

  React.useEffect(() => {
    if (props?.open !== undefined) setIsOpen(props.open);
  }, [props?.open]);

  const handleOpenChange = React.useCallback(
    (open: boolean) => {
      setIsOpen(open);
      props.onOpenChange?.(open);
    },
    [props],
  );

  return (
    <CollapsibleContext.Provider value={{ isOpen }}>
      <CollapsiblePrimitive.Root
        data-slot="collapsible"
        {...props}
        onOpenChange={handleOpenChange}
      >
        {children}
      </CollapsiblePrimitive.Root>
    </CollapsibleContext.Provider>
  );
}

type CollapsibleTriggerProps = React.ComponentProps<
  typeof CollapsiblePrimitive.Trigger
>;

function CollapsibleTrigger(props: CollapsibleTriggerProps) {
  return (
    <CollapsiblePrimitive.Trigger data-slot="collapsible-trigger" {...props} />
  );
}

type CollapsibleContentProps = React.ComponentProps<
  typeof CollapsiblePrimitive.Content
>;

function CollapsibleContent({
  className,
  children,
  ...props
}: CollapsibleContentProps) {
  return (
    <CollapsiblePrimitive.Content
      data-slot="collapsible-content"
      className={className}
      {...props}
    >
      {children}
    </CollapsiblePrimitive.Content>
  );
}

export {
  Collapsible,
  CollapsibleTrigger,
  CollapsibleContent,
  useCollapsible,
  type CollapsibleContextType,
  type CollapsibleProps,
  type CollapsibleTriggerProps,
  type CollapsibleContentProps,
};
