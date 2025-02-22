//
//  ShareView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation

public struct ShareView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @StateObject private var core = Core()
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
                                    .color(Theme.SecondaryVividViolet.color),
                                    .color(Theme.DarkViolet.color),
                                    .float(elapsedTime * 0.5)
                                )
                            )
                    }
            }

            VStack {
                Spacer()
                LogoView(width: 65)
                    .frame(width: .infinity, height: 100)
                Text("Secure and Fastest \n file transfer")
                    .multilineTextAlignment(.center)
                    .modifier(Label1())
                    .foregroundStyle(Theme.gradientHeading)
                    .padding(.top, 10)
                UpgradePremiumButton()
                Spacer()
                HStack {
                    Text("Add new file")
                        .modifier(Body2())
                        .opacity(0.5)
                    AddFileButton()
                }
                Spacer()
                Spacer()
                Spacer()
                Spacer()
                Spacer()
            }
            ShareButton(width: 350)
                .padding(.bottom, 60)
        }
        .ignoresSafeArea()
    }
}

#Preview {
    ShareView()
}
