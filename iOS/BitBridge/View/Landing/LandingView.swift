//
//  LandingView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct LandingView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @EnvironmentObject private var core: Core

    var body: some View {
        VStack(alignment: .leading) {
            LogoView(width: 10)
                .frame(width: screenSize.width, height: screenSize.width)
                .overlay(Theme.gradientHeading.opacity(1).blur(radius: 30).frame(width: .infinity, height: screenSize.width / 2).offset(y: screenSize.width / 4))
                .padding(.top, safeAreaInsets.top)

            Text("We feel thankful that you’re here 🙌")
                .foregroundColor(Theme.LightSea.color)
                .modifier(Label1())
                .padding(.top, 20)
                .padding(.horizontal, SpaceTheme.screen.value)

            Text("The most\nsecure and fastest\nfile transfer 🚀")
                .modifier(H1Heading())
                .padding(.top, 10)
                .padding(.horizontal, SpaceTheme.screen.value)

            Spacer()

            Button(action: {
                Task {
                    await core.update(.authentication(.signIn))
                }
            }) {
                Text("Get started")
            }
            .padding(.horizontal, SpaceTheme.screen.value)
            .buttonStyle(PrimaryButtonStyle())
        }
        .ignoresSafeArea(.all)
        .padding(.bottom, safeAreaInsets.bottom)
        .background(StunningBackgroundGradient())
    }
}

struct LandingView_Preview: PreviewProvider {
    static var previews: some View {
        LandingView()
    }
}
