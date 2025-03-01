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
            // Bottom glow
            TimelineView(.animation) { timeline in
                let elapsedTime = startTime.distance(to: timeline.date)
                Rectangle()
                    .frame(width: screenSize.width, height: screenSize.height + safeAreaInsets.bottom)
                    .visualEffect { content, proxy in
                        content
                            .colorEffect(
                                ShaderLibrary.generateBackground(
                                    .float2(proxy.size),
                                    .color(Theme.SecondaryViolet.color.opacity(0.5)),
                                    .color(Theme.DarkViolet.color.opacity(0.9)),
                                    .float(elapsedTime * 0.6)
                                )
                            )
                    }
            }
            
            // Top glow (with different colors and timing)
            TimelineView(.animation) { timeline in
                let elapsedTime = startTime.distance(to: timeline.date)
                Rectangle()
                    .frame(width: screenSize.width, height: screenSize.height + safeAreaInsets.bottom)
                    .visualEffect { content, proxy in
                        content
                            .colorEffect(
                                ShaderLibrary.generateBackground(
                                    .float2(proxy.size),
                                    .color(Theme.Pink.color.opacity(0.5)),
                                    .color(Theme.SecondaryBlue.color.opacity(0.2)),
                                    .float(elapsedTime * 0.6)
                                )
                            )
                    }
                    .rotationEffect(.degrees(180)) // Flip to create different movement pattern
                    .opacity(0.5)
            }
        }
    }
}

struct StunningBackgroundGradient: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize

    public var body: some View {
        ZStack {
            Circle()
                .frame(width: screenSize.width / 2, height: screenSize.width / 2)
                .position(x: screenSize.width, y: 0)
                .foregroundStyle(Theme.SeaTertiary.color.opacity(0.4))
                .blur(radius: 150)
            Circle()
                .frame(width: screenSize.width, height: screenSize.width)
                .foregroundStyle(Theme.SeaTertiary.color.opacity(0.2))
                .blur(radius: 200)
                .position(x: 0, y: screenSize.height)
        }
        .background(Theme.BlackBase.color)
    }
}

#Preview {
    StunningBackgroundGradientAnimation()
}

#Preview {
    StunningBackgroundGradient()
        .frame(width: .infinity, height: .infinity)
}
