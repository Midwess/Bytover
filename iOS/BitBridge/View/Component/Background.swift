//
//  Background.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 22/2/25.
//

import Foundation
import SwiftUI


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
                .foregroundStyle(Theme.SeaTertiary.color.opacity(0.4))
                .blur(radius: 150)
                .position(x: 0, y: screenSize.height)
        }
        .background(Theme.BlackBase.color)
    }
}

struct StunningBackgroundGradientSecondary: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize

    public var body: some View {
        ZStack {
            Circle()
                .frame(width: 150, height: 150)
                .position(x: screenSize.width / 2, y: -10)
                .background(Theme.BlackSecondary.color.opacity(0.0))
                .foregroundStyle(Theme.GreenSecondary.color.opacity(1))
                .blur(radius: 120)
        }
        .background(Theme.BlackBase.color)
    }
}

struct FadingBackground: View {
    @State private var opacity: Double
    
    private let finalOpacity: Double
    private let duration: Double
    private let autoStart: Bool
    
    init(
        initialOpacity: Double = 1.0,
        finalOpacity: Double = 0.0,
        duration: Double = 1.5,
        autoStart: Bool = true
    ) {
        self._opacity = State(initialValue: initialOpacity)
        self.finalOpacity = finalOpacity
        self.duration = duration
        self.autoStart = autoStart
    }
    
    var body: some View {
        Rectangle()
            .fill(Theme.BlackBase.color)
            .ignoresSafeArea()
            .opacity(opacity)
            .onAppear {
                if autoStart {
                    startAnimation()
                }
            }
    }
    
    func startAnimation() {
        withAnimation(.easeInOut(duration: duration)) {
            opacity = finalOpacity
        }
    }
}

#Preview {
    StunningBackgroundGradientSecondary()
}

#Preview {
    FadingBackground()
}

#Preview {
    StunningBackgroundGradient()
        .frame(width: .infinity, height: .infinity)
}
