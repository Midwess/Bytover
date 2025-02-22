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

    public var body: some View {
        ZStack(alignment: .bottom) {
            Theme.darkBgGradient.opacity(0.3)
            TimelineView(.animation) { timeline in
                let elapsedTime = startTime.distance(to: timeline.date)
                Rectangle()
                    .frame(width: .infinity, height: .infinity)
                    .visualEffect { content, proxy in
                        content
                            .colorEffect(
                                ShaderLibrary.generateBackground(
                                    .float2(proxy.size),
                                    .color(Theme.SecondaryViolet.color.opacity(0.5)),
                                    .color(Theme.DarkViolet.color.opacity(0.3)),
                                    .float(elapsedTime * 0.5)
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
