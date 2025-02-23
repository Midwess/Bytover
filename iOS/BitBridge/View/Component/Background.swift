//
//  Background.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 22/2/25.
//

import Foundation
import SwiftUI

struct StunningBackgroundGradientAnimation: View {
    @State private var startTime = Date.now
    @Environment(\.screenSize) private var screenSize
    @Environment(\.safeAreaInsets) private var safeAreaInsets

    public var body: some View {
        ZStack(alignment: .bottom) {
            TimelineView(.animation) { timeline in
                let elapsedTime = startTime.distance(to: timeline.date)
                Rectangle()
                    .frame(width: screenSize.width, height: screenSize.height + safeAreaInsets.bottom)
                    .visualEffect { content, proxy in
                        content
                            .colorEffect(
                                ShaderLibrary.generateBackground(
                                    .float2(proxy.size),
                                    .color(Theme.LightPrimaryViolet.color.opacity(0.9)),
                                    .color(Theme.DarkViolet.color.opacity(0.3)),
                                    .float(elapsedTime * 0.6)
                                )
                            )
                    }
            }
        }
    }
}

#Preview {
    StunningBackgroundGradientAnimation()
}
